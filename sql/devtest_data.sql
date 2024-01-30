use posted_mysql;

INSERT INTO Account (username, password_hash) VALUES
    ("devtest_1", "super_secret"),   -- 1
    ("devtest_2", "super_secret2"),  -- 2
    ("devtest_3", "super_secret3");  -- 3

INSERT INTO Post (poster_id, title, body) VALUES
    (1, "test_post_1", "abrakadabra"), -- 1
    (1, "test_post_2", "another one by devtest user 1");  -- 2

INSERT INTO Comment (post_id, commenter_id, body) VALUES
    (1, 2, "A comment under post 1 by devtest_2"),  -- 1
    (1, 3, "A comment under post 1 by devtest_3"),  -- 2
    (2, 1, "A comment by devtest_1 under their own post (2)");  -- 3
    
INSERT INTO PostLike (post_id, account_id) VALUES
    (1, 1), (1, 2), (1, 3),
    (2, 1);

INSERT INTO CommentLike (comment_id, account_id) VALUES 
    (1, 1), (1, 2),
    (2, 3),
    (3, 1);