use posted_mysql;

DROP TABLE IF EXISTS PostLike;
DROP TABLE IF EXISTS CommentLike;
DROP TABLE IF EXISTS Comment;
DROP TABLE IF EXISTS Post;
DROP TABLE IF EXISTS Account;

CREATE TABLE Account (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    username VARCHAR(127) NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    PRIMARY KEY (id),
    UNIQUE (username)
);

CREATE TABLE Post (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    poster_id BIGINT UNSIGNED NOT NULL,
    title VARCHAR(127) NOT NULL,
    body VARCHAR(1024) NOT NULL,
    time_stamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP(), -- TIMESTAMP is UTC
    edited BOOLEAN DEFAULT false,
    PRIMARY KEY (id),
    FOREIGN KEY (poster_id) REFERENCES Account(id)
);

CREATE TABLE Comment (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    post_id BIGINT UNSIGNED NOT NULL,
    commenter_id BIGINT UNSIGNED NOT NULL,
    body VARCHAR(255),
    comment_reply_id BIGINT UNSIGNED,
    time_stamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP(), -- TIMESTAMP is UTC
    edited BOOLEAN DEFAULT false,
    PRIMARY KEY (id),
    FOREIGN KEY (post_id) REFERENCES Post(id),
    FOREIGN KEY (commenter_id) REFERENCES Account(id),
    FOREIGN KEY (comment_reply_id) REFERENCES Comment(id)
);

CREATE TABLE PostLike (
    post_id BIGINT UNSIGNED NOT NULL,
    account_id BIGINT UNSIGNED NOT NULL,
    PRIMARY KEY (post_id, account_id),
    FOREIGN KEY (post_id) REFERENCES Post(id),
    FOREIGN KEY (account_id) REFERENCES Account(id)
);

CREATE TABLE CommentLike (
    comment_id BIGINT UNSIGNED NOT NULL,
    account_id BIGINT UNSIGNED NOT NULL,
    PRIMARY KEY (comment_id, account_id),
    FOREIGN KEY (comment_id) REFERENCES Comment(id),
    FOREIGN KEY (account_id) REFERENCES Account(id)
);