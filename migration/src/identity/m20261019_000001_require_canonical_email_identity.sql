-- An email is stored in canonical form: no surrounding whitespace and lower case
-- (specs/identity/sign-in, FR5 and Q5). The application normalises before it looks up or stores
-- an email; this constraint makes sure no writer can store anything else.
--
-- btrim is given the exact characters that Rust's `str::trim` removes (Unicode White_Space), so
-- the code and the database agree on what "trimmed" means. The plain btrim(email) would remove
-- spaces only.
ALTER TABLE identity.users
    ADD CONSTRAINT users_email_canonical
    CHECK (email = lower(btrim(email, E' \t\n\u000b\u000c\r\u0085\u00a0\u1680\u2000\u2001\u2002\u2003\u2004\u2005\u2006\u2007\u2008\u2009\u200a\u2028\u2029\u202f\u205f\u3000')));
