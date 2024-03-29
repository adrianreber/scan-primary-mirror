use crate::db::schema::{
    category, category_directory, directory, file_detail, repository, version,
};

#[derive(Queryable, Identifiable, Associations)]
#[diesel(belongs_to(Directory, foreign_key = topdir_id))]
#[diesel(table_name = category)]
pub struct Category {
    pub id: i32,
    pub name: String,
    pub topdir_id: i32,
    pub product_id: i32,
}

#[derive(Queryable, Identifiable, Clone, Debug)]
#[diesel(table_name = directory)]
pub struct Directory {
    pub id: i32,
    pub name: String,
    pub files: Vec<u8>,
    pub readable: bool,
    pub ctime: i64,
}

#[derive(Queryable, Identifiable)]
#[diesel(primary_key(category_id))]
#[diesel(primary_key(directory_id))]
#[diesel(table_name = category_directory)]
pub struct CategoryDirectory {
    pub category_id: i32,
    pub directory_id: i32,
}

#[derive(Queryable)]
pub struct Arch {
    pub id: i32,
    pub name: String,
}

#[derive(Queryable, Identifiable, Debug, Clone)]
#[diesel(table_name = version)]
pub struct Version {
    pub id: i32,
    pub name: String,
    pub product_id: i32,
    pub is_test: bool,
}

#[derive(Queryable, Identifiable, Debug)]
#[diesel(table_name = version)]
pub struct InsertVersion {
    pub id: i32,
    pub name: String,
    pub product_id: i32,
    pub is_test: bool,
    pub sortorder: i32,
    pub display: bool,
    pub ordered_mirrorlist: bool,
}

#[derive(Queryable, Identifiable, Debug)]
#[diesel(table_name = repository)]
pub struct Repository {
    pub id: i32,
    pub name: String,
    pub prefix: Option<String>,
    pub category_id: Option<i32>,
    pub version_id: Option<i32>,
    pub arch_id: Option<i32>,
    pub directory_id: Option<i32>,
    pub disabled: bool,
}

#[derive(Queryable, Debug, Clone, Eq, PartialEq)]
pub struct FileDetail {
    pub id: i32,
    pub directory_id: i32,
    pub filename: String,
    pub timestamp: Option<i64>,
    pub size: Option<i64>,
    pub sha1: Option<String>,
    pub md5: Option<String>,
    pub sha256: Option<String>,
    pub sha512: Option<String>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = file_detail)]
pub struct InsertFileDetail {
    pub directory_id: i32,
    pub filename: String,
    pub timestamp: Option<i64>,
    pub size: Option<i64>,
    pub sha1: Option<String>,
    pub md5: Option<String>,
    pub sha256: Option<String>,
    pub sha512: Option<String>,
}
