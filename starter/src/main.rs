use arc_swap::ArcSwap;
use axum::{body::{to_bytes, Body, Bytes}, extract::{Request, State}, http::{Method, header}, response::Response, Router};
use chrono::{Utc, SecondsFormat};
use rusqlite::{Connection, params, OptionalExtension};
use serde::Serialize;
use serde_json::{json, Value};
use std::{collections::{VecDeque, HashMap}, env, sync::Arc};
use tokio::sync::{mpsc, oneshot};

#[derive(Clone, Serialize)]
struct Post { id: i64, author: String, content: String, created_at: String, likes: i64 }
type Reply = (u16, Bytes);
struct Command { op: Operation, reply: oneshot::Sender<Reply> }
enum Operation { Post(String, String), Like(i64, String), Health }
struct Shared { writer: mpsc::Sender<Command>, feed: Arc<ArcSwap<Bytes>> }
fn encoded(v: impl Serialize) -> Bytes { Bytes::from(serde_json::to_vec(&v).expect("JSON serialization")) }
fn error(status: u16, msg: &str) -> Reply { (status, encoded(json!({"error": msg}))) }
fn response(r: Reply, head: bool) -> Response {
    let len = r.1.len();
    Response::builder().status(r.0).header(header::CONTENT_TYPE, "application/json")
        .header(header::CONTENT_LENGTH, len).body(if head { Body::empty() } else { Body::from(r.1) }).unwrap()
}
fn name(v: Option<&Value>) -> Option<String> {
    let s = v?.as_str()?;
    if (1..=32).contains(&s.len()) && s.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_') { Some(s.to_owned()) } else { None }
}
type Input = HashMap<String, Box<serde_json::value::RawValue>>;
fn field(value: &Input, key: &str) -> Option<Value> {
    serde_json::from_str(value.get(key)?.get()).ok()
}
fn feed_bytes(posts: &VecDeque<Post>) -> Bytes { encoded(json!({"posts":posts})) }
fn initial(db: &Connection) -> rusqlite::Result<VecDeque<Post>> {
    let mut stmt = db.prepare("SELECT p.id,p.author,p.content,p.created_at,(SELECT COUNT(*) FROM likes l WHERE l.post_id=p.id) FROM posts p ORDER BY p.id DESC LIMIT 20")?;
    let posts = stmt.query_map([], |r| Ok(Post{id:r.get(0)?,author:r.get(1)?,content:r.get(2)?,created_at:r.get(3)?,likes:r.get(4)?}))?.collect();
    posts
}
fn write_post(db: &mut Connection, author: String, content: String) -> rusqlite::Result<Post> {
    let created_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    let tx = db.transaction()?;
    tx.prepare_cached("INSERT INTO posts(author,content,created_at) VALUES(?,?,?)")?.execute(params![author,content,created_at])?;
    let id = tx.last_insert_rowid();
    tx.commit()?; // FULL WAL sync is complete before any cache publication or reply.
    Ok(Post{id,author,content,created_at,likes:0})
}
fn write_like(db: &mut Connection, id: i64, user: String) -> rusqlite::Result<Option<i64>> {
    let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let exists = tx.prepare_cached("SELECT 1 FROM posts WHERE id=?")?.query_row([id], |_| Ok(())).optional()?.is_some();
    if !exists { tx.commit()?; return Ok(None); }
    tx.prepare_cached("INSERT OR IGNORE INTO likes(post_id,user) VALUES(?,?)")?.execute(params![id,user])?;
    let count = tx.prepare_cached("SELECT COUNT(*) FROM likes WHERE post_id=?")?.query_row([id], |r| r.get(0))?;
    tx.commit()?;
    Ok(Some(count))
}
fn writer(mut db: Connection, mut posts: VecDeque<Post>, snapshot: Arc<ArcSwap<Bytes>>, mut rx: mpsc::Receiver<Command>) {
    while let Some(cmd) = rx.blocking_recv() {
        let result: rusqlite::Result<Reply> = match cmd.op {
            Operation::Health => db.prepare_cached("SELECT 1").and_then(|mut s| s.query_row([], |_| Ok((200, encoded(json!({"status":"ok"})))) )),
            Operation::Post(author, content) => write_post(&mut db, author, content).map(|post| {
                let bytes = encoded(&post);
                posts.push_front(post); posts.truncate(20);
                snapshot.store(Arc::new(feed_bytes(&posts)));
                (201, bytes)
            }),
            Operation::Like(id, user) => write_like(&mut db, id, user).map(|count| match count {
                None => error(404,"Post not found"),
                Some(likes) => {
                    if let Some(post) = posts.iter_mut().find(|p| p.id == id) {
                        if post.likes != likes { post.likes = likes; snapshot.store(Arc::new(feed_bytes(&posts))); }
                    }
                    (200, encoded(json!({"post_id":id,"likes":likes,"liked":true})))
                }
            })
        };
        let reply = result.unwrap_or_else(|e| { eprintln!("database operation failed: {e}"); error(500,"Internal Server Error") });
        // Snapshot publication follows COMMIT and precedes acknowledgment. One writer
        // serializes DB state, snapshot generations, and responses without group commit.
        let _ = cmd.reply.send(reply); // Disconnected client does not undo committed data.
    }
}
async fn dispatch(state: &Shared, op: Operation) -> Reply {
    let (reply, rx) = oneshot::channel();
    if state.writer.send(Command{op,reply}).await.is_err() { return error(500,"Internal Server Error"); }
    rx.await.unwrap_or_else(|_| error(500,"Internal Server Error"))
}
async fn handle(State(state): State<Arc<Shared>>, req: Request) -> Response {
    let method = req.method().clone();
    let head = method == Method::HEAD;
    let path = percent_encoding::percent_decode_str(req.uri().path()).decode_utf8_lossy();
    let read = matches!(path.as_ref(), "/feed" | "/health");
    let write = matches!(path.as_ref(), "/posts" | "/like");
    if !read && !write { return response(error(404,"The requested URL was not found on the server. If you entered the URL manually please check your spelling and try again."), head); }
    let allow = if read { "GET, HEAD, OPTIONS" } else { "POST, OPTIONS" };
    if method == Method::OPTIONS { return Response::builder().status(200).header(header::ALLOW,allow).header(header::CONTENT_TYPE,"text/html; charset=utf-8").header(header::CONTENT_LENGTH,0).body(Body::empty()).unwrap(); }
    if !(read && (method == Method::GET || head) || write && method == Method::POST) {
        return response(error(405,"The method is not allowed for the requested URL."),head);
    }
    if path == "/feed" { return response((200,(**state.feed.load()).clone()),head); }
    if path == "/health" { return response(dispatch(&state,Operation::Health).await,head); }
    let like = path == "/like";
    let mime = req.headers().get(header::CONTENT_TYPE).and_then(|v| v.to_str().ok()).unwrap_or("").split(';').next().unwrap_or("").trim().to_ascii_lowercase();
    if !(mime == "application/json" || (mime.starts_with("application/") && mime.ends_with("+json"))) {
        return response(error(400,"Expected a JSON object"), false);
    }
    let bytes = match to_bytes(req.into_body(),16384).await { Ok(v)=>v, Err(_)=>return response(error(400,"The data value transmitted exceeds the capacity limit."),false) };
    // Decode used fields only; preserve the semantics of ignored extra keys.
    let input = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(&bytes);
    let value: Input = match serde_json::from_slice(input) { Ok(v)=>v, _=>return response(error(400,"Expected a JSON object"),false) };
    let op = if like {
        let post_id = field(&value,"post_id");
        let user = field(&value,"user");
        match (post_id.as_ref().and_then(Value::as_i64).filter(|id| *id>0),name(user.as_ref())) {
            (Some(id),Some(user))=>Operation::Like(id,user),
            _=>return response(error(400,"Expected a positive integer post_id and a valid user"),false)
        }
    } else {
        let author_value=field(&value,"author");
        let author=match name(author_value.as_ref()) { Some(v)=>v, None=>return response(error(400,"author must match [a-zA-Z0-9_]{1,32}"),false) };
        let content_value=field(&value,"content");
        if content_value.is_none() && value.get("content").is_some_and(|raw| raw.get().starts_with('"')) {
            return response(error(400,"content must be valid UTF-8"),false);
        }
        let content=match content_value.as_ref().and_then(Value::as_str) {
            Some(v) if (1..=280).contains(&v.chars().count()) && v.chars().any(|c| !(c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)))=>v.to_owned(),
            _=>return response(error(400,"content must contain 1..280 characters and not be blank"),false)
        };
        Operation::Post(author,content)
    };
    response(dispatch(&state,op).await,false)
}
#[tokio::main(flavor="multi_thread", worker_threads=2)]
async fn main() {
    let dir=env::var("DATA_DIR").unwrap_or_else(|_|"/data".into());
    std::fs::create_dir_all(&dir).expect("create DATA_DIR");
    let db=Connection::open(std::path::Path::new(&dir).join("social.sqlite3")).expect("open database");
    db.busy_timeout(std::time::Duration::from_secs(10)).expect("busy timeout");
    db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; CREATE TABLE IF NOT EXISTS posts(id INTEGER PRIMARY KEY AUTOINCREMENT,author TEXT NOT NULL,content TEXT NOT NULL,created_at TEXT NOT NULL); CREATE TABLE IF NOT EXISTS likes(post_id INTEGER NOT NULL,user TEXT NOT NULL); CREATE UNIQUE INDEX IF NOT EXISTS idx_likes_post_user ON likes(post_id,user);").expect("schema and durability pragmas");
    let mode:String=db.pragma_query_value(None,"journal_mode",|r|r.get(0)).unwrap();
    let sync:i64=db.pragma_query_value(None,"synchronous",|r|r.get(0)).unwrap();
    assert_eq!(mode,"wal"); assert_eq!(sync,2);
    eprintln!("SQLite journal_mode={mode} synchronous={sync} writer=single commit=per-operation");
    let posts=initial(&db).expect("load committed feed");
    let feed=Arc::new(ArcSwap::from_pointee(feed_bytes(&posts)));
    let (tx,rx)=mpsc::channel(4096);
    let writer_feed=feed.clone();
    std::thread::Builder::new().name("sqlite-durable-writer".into()).spawn(move||writer(db,posts,writer_feed,rx)).unwrap();
    let state=Arc::new(Shared{writer:tx,feed});
    let port=env::var("PORT").unwrap_or_else(|_|"8080".into());
    let listener=tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await.expect("listen");
    axum::serve(listener,Router::new().fallback(handle).with_state(state)).await.expect("serve");
}
