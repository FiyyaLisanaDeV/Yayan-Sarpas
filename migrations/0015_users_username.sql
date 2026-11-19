-- Login tanpa email: kolom username sebagai identitas utama masuk.
-- Email tetap dipakai untuk administrasi; lookup login menerima keduanya.
ALTER TABLE users ADD COLUMN IF NOT EXISTS username TEXT;

UPDATE users SET username = lower(split_part(email, '@', 1))
WHERE username IS NULL OR username = '';

ALTER TABLE users ALTER COLUMN username SET NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS users_username_lower_unique ON users (lower(username));
