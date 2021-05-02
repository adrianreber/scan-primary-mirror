table! {
    directory (id) {
        id -> Integer,
        name -> Text,
        files -> Binary,
        readable -> Bool,
        ctime -> BigInt,
    }
}

table! {
    category (id) {
        id -> Integer,
        name -> Text,
        topdir_id -> Integer,
        product_id -> Integer,
    }
}

table! {
    category_directory (category_id, directory_id) {
        category_id -> Integer,
        directory_id -> Integer,
        ctime -> BigInt,
    }
}

table! {
    arch (id) {
        id -> Integer,
        name -> Text,
    }
}

table! {
    version (id) {
        id -> Integer,
        name -> Text,
        product_id -> Integer,
        is_test -> Bool,
        sortorder -> Integer,
        display -> Bool,
        ordered_mirrorlist -> Bool,
    }
}

table! {
    repository (id) {
        id -> Integer,
        name -> Text,
        prefix -> Nullable<Text>,
        category_id -> Nullable<Integer>,
        version_id -> Nullable<Integer>,
        arch_id -> Nullable<Integer>,
        directory_id -> Nullable<Integer>,
        disabled -> Bool,
    }
}

table! {
    file_detail (id) {
        id -> Integer,
        directory_id -> Integer,
        filename -> Text,
        timestamp -> Nullable<BigInt>,
        size -> Nullable<BigInt>,
        sha1 -> Nullable<Text>,
        md5 -> Nullable<Text>,
        sha256 -> Nullable<Text>,
        sha512 -> Nullable<Text>,
    }
}

joinable!(category -> directory (topdir_id));

allow_tables_to_appear_in_same_query!(category, directory);
