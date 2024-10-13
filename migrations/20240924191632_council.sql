-- Add migration script here
CREATE TABLE council (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    secretary_email VARCHAR(255) NOT NULL,
    deputy_secretaries_email VARCHAR(255)[] NOT NULL,
    FOREIGN KEY (secretary_email) REFERENCES user_profile(email)
);