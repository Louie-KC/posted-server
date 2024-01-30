use posted_mysql;

DROP TABLE IF EXISTS PostLike;
DROP TABLE IF EXISTS CommentLike;
DROP TABLE IF EXISTS Comment;
DROP TABLE IF EXISTS Account;
DROP TABLE IF EXISTS Post;

CREATE TABLE Account (
    id INT NOT NULL AUTO_INCREMENT,
    username VARCHAR(127) NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    PRIMARY KEY (id),
    UNIQUE (username)
);

CREATE TABLE Post (
    id INT NOT NULL AUTO_INCREMENT,
    poster_id INT NOT NULL,
    title VARCHAR(127) NOT NULL,
    body VARCHAR(1024) NOT NULL,
    post_time TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP(),
    PRIMARY KEY (id),
    FOREIGN KEY (poster_id) REFERENCES Account(id)
);

CREATE TABLE Comment (
    id INT NOT NULL AUTO_INCREMENT,
    post_id INT NOT NULL,
    commenter_id INT NOT NULL,
    body VARCHAR(255),
    comment_reply_id INT,
    PRIMARY KEY (id),
    FOREIGN KEY (post_id) REFERENCES Post(id),
    FOREIGN KEY (commenter_id) REFERENCES Account(id),
    FOREIGN KEY (comment_reply_id) REFERENCES Comment(id)
);

CREATE TABLE PostLike (
    post_id INT NOT NULL,
    account_id INT NOT NULL,
    PRIMARY KEY (post_id, account_id),
    FOREIGN KEY (post_id) REFERENCES Post(id),
    FOREIGN KEY (account_id) REFERENCES Account(id)
);

CREATE TABLE CommentLike (
    comment_id INT NOT NULL,
    account_id INT NOT NULL,
    PRIMARY KEY (comment_id, account_id),
    FOREIGN KEY (comment_id) REFERENCES Comment(id),
    FOREIGN KEY (account_id) REFERENCES Account(id)
);