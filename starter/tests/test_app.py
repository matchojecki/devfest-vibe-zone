import os
import tempfile
import unittest
import sqlite3
from pathlib import Path
import sys

# Set temporary data dir before importing app
TEMP_DIR = tempfile.TemporaryDirectory()
os.environ["DATA_DIR"] = TEMP_DIR.name

# Add starter to path
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import app
from app import app as flask_app, DB_PATH, connection

class TestOptimizedApp(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        pass

    @classmethod
    def tearDownClass(cls):
        TEMP_DIR.cleanup()

    def setUp(self):
        flask_app.config['TESTING'] = True
        self.client = flask_app.test_client()
        with flask_app.app_context():
            db = connection()
            db.execute("DELETE FROM likes")
            db.execute("DELETE FROM posts")
            db.commit()

    def test_database_pragmas_and_unique_index(self):
        with flask_app.app_context():
            db = connection()
            
            # Check journal_mode = wal
            journal_mode = db.execute("PRAGMA journal_mode").fetchone()[0].lower()
            self.assertEqual(journal_mode, "wal")

            # Check synchronous = FULL (2)
            sync = db.execute("PRAGMA synchronous").fetchone()[0]
            self.assertEqual(sync, 2)

            # Check UNIQUE index on likes(post_id, user)
            indexes = db.execute("PRAGMA index_list(likes)").fetchall()
            unique_index_found = False
            for idx in indexes:
                if idx["unique"]:
                    cols = db.execute(f"PRAGMA index_info({idx['name']})").fetchall()
                    col_names = [c["name"] for c in cols]
                    if col_names == ["post_id", "user"]:
                        unique_index_found = True
                        break
            self.assertTrue(unique_index_found, "Expected UNIQUE index on likes(post_id, user)")

    def test_feed_single_query_aggregation(self):
        # Insert 25 posts and some likes
        with flask_app.app_context():
            db = connection()
            for i in range(25):
                db.execute("INSERT INTO posts (author, content, created_at) VALUES (?, ?, ?)",
                           (f"user_{i}", f"content_{i}", f"2026-11-21T12:00:{i:02d}.000Z"))
            db.commit()
            post_id = db.execute("SELECT id FROM posts ORDER BY id DESC LIMIT 1").fetchone()[0]
            db.execute("INSERT INTO likes (post_id, user) VALUES (?, ?)", (post_id, "alice"))
            db.commit()

        # Call GET /feed under preserved Flask app context with trace_callback
        queries = []
        def trace(sql):
            queries.append(sql)

        with flask_app.app_context():
            db = connection()
            db.set_trace_callback(trace)
            res = self.client.get("/feed")

        self.assertEqual(res.status_code, 200)
        data = res.get_json()
        self.assertIn("posts", data)
        self.assertEqual(len(data["posts"]), 20)
        # Check that likes are correctly aggregated
        found_liked_post = False
        for p in data["posts"]:
            if p["id"] == post_id:
                self.assertEqual(p["likes"], 1)
                found_liked_post = True
        self.assertTrue(found_liked_post)

        # Count SELECT (including WITH if relevant) and assert 1
        select_count = sum(1 for q in queries if q.strip().upper().startswith(("SELECT", "WITH")))
        self.assertEqual(select_count, 1)

    def test_like_idempotency(self):
        # Create post
        res = self.client.post("/posts", json={"author": "alice", "content": "test post"})
        self.assertEqual(res.status_code, 201)
        post_id = res.get_json()["id"]

        # Like multiple times by same user
        for _ in range(3):
            res = self.client.post("/like", json={"post_id": post_id, "user": "bob"})
            self.assertEqual(res.status_code, 200)
            self.assertEqual(res.get_json()["likes"], 1)
            self.assertTrue(res.get_json()["liked"])

if __name__ == "__main__":
    unittest.main()
