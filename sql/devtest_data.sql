use posted_mysql;

-- (Dev)Test ID/PK range: 0..=100

INSERT INTO Account (id, username, password_hash) VALUES
    (1, "devtest_1", "super_secret"),
    (2, "devtest_2", "super_secret2"),
    (3, "devtest_3", "super_secret3");

INSERT INTO Post (id, poster_id, title, body) VALUES
    (1, 1, "test_post_1", "abrakadabra"),
    (2, 1, "test_post_2", "another one by devtest user 1");

INSERT INTO Comment (id, post_id, commenter_id, body) VALUES
    (1, 1, 2, "A comment under post 1 by devtest_2"),
    (2, 1, 3, "A comment under post 1 by devtest_3"),
    (3, 2, 1, "A comment by devtest_1 under their own post (2)");
    
INSERT INTO PostLike (post_id, account_id) VALUES
    (1, 1), (1, 2), (1, 3),
    (2, 1);

INSERT INTO CommentLike (comment_id, account_id) VALUES 
    (1, 1), (1, 2),
    (2, 3),
    (3, 1);