-- Add migration script here
CREATE TABLE transportation (
    id INTEGER PRIMARY KEY,
    start_time TIMESTAMP WITH TIME ZONE NOT NULL,
    end_time TIMESTAMP WITH TIME ZONE NOT NULL,
    stops VARCHAR(255)[] NOT NULL,
    service_start_time TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT (now() at time zone 'utc'),
    service_end_time TIMESTAMP WITH TIME ZONE
);