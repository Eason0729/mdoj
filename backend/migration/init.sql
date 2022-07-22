CREATE TABLE user_table(
	id SERIAL NOT NULL PRIMARY KEY,
	create_time DATE,
	update_time DATE,
	name_user VARCHAR(64) NOT NULL,
	privilege INTEGER DEFAULT 3 NOT NULL,
	hashed_password bytea NOT NULL,
	salt bytea NOT NULL,
	description VARCHAR(512) DEFAULT ''
);
CREATE TABLE token_table(
	id SERIAL NOT NULL PRIMARY KEY,
    user_id INT NOT NULL,
	CONSTRAINT FK_user FOREIGN KEY (user_id) REFERENCES user_table(id),
	ttl bigint NOT NULL,
	key bytea NOT NULL,
	data bytea NOT NULL
);
CREATE TABLE session_table(
	id SERIAL NOT NULL PRIMARY KEY, 
	key VARCHAR(64),
	data bytea NOT NULL
);
CREATE TABLE question_table(
	id SERIAL NOT NULL PRIMARY KEY,
	title VARCHAR(128) DEFAULT '',
	description VARCHAR(1024) DEFAULT ''
);
CREATE TABLE question_user(
	id SERIAL NOT NULL PRIMARY KEY,
    user_id INT NOT NULL,
    question_id INT NOT NULL,
	CONSTRAINT FK_user FOREIGN KEY (user_id) REFERENCES user_table(id),
	CONSTRAINT FK_question FOREIGN KEY (question_id) REFERENCES question_table(id)
);
CREATE TABLE group_table(
	id SERIAL NOT NULL PRIMARY KEY,
	data bytea NOT NULL
);
CREATE TABLE group_user(
	id SERIAL NOT NULL PRIMARY KEY,
    user_id INT NOT NULL,
    group_id INT NOT NULL,
	CONSTRAINT FK_user FOREIGN KEY (user_id) REFERENCES user_table(id),
	CONSTRAINT FK_group FOREIGN KEY (group_id) REFERENCES group_table(id)
);