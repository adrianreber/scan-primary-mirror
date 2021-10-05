DROP TABLE file_detail;

CREATE TABLE file_detail (
    id serial NOT NULL,
    directory_id integer NOT NULL,
    filename text NOT NULL,
    "timestamp" bigint,
    size bigint,
    sha1 text,
    md5 text,
    sha256 text,
    sha512 text
);

DROP TABLE directory;
CREATE TABLE directory (
    id serial NOT NULL,
    name text NOT NULL,
    files bytea,
    readable boolean,
    ctime bigint
);

DROP TABLE category_directory;
CREATE TABLE category_directory (
    category_id integer NOT NULL,
    directory_id integer NOT NULL,
    ctime bigint
);

DROP TABLE version;
CREATE TABLE version (
    id serial NOT NULL,
    name text,
    product_id integer,
    is_test boolean,
    display boolean,
    display_name text,
    ordered_mirrorlist boolean,
    sortorder integer NOT NULL,
    codename text
);

DROP TABLE arch;
CREATE TABLE arch (
    id serial NOT NULL,
    name text
);
