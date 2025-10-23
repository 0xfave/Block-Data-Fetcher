-- Increase transaction_label column size from 100 to 255 characters
-- Labels can be long when they include program names, e.g.:
-- "Token Transfer (Token Program, Associated Token Program, System Program)"

ALTER TABLE transactions 
ALTER COLUMN transaction_label TYPE VARCHAR(255);
