-- Add migration script here
CREATE TABLE comment_likes (
    comment_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    PRIMARY KEY (comment_id, user_id),
    FOREIGN KEY (comment_id) REFERENCES comment(id),
    FOREIGN KEY (user_id) REFERENCES user_profile(id)
);