// SPDX-License-Identifier: MIT

use crate::db::models::{Arch, CategoryDirectory, Directory, FileDetail, Repository, Version};
use crate::debug::*;

use diesel::pg::PgConnection;
use diesel::prelude::*;

#[derive(Debug, Default, Queryable, Clone)]
pub struct Category {
    pub id: i32,
    pub name: String,
    pub topdir: String,
    pub product_id: i32,
}

/// Retrieve the list of architectures from the database
///
/// Architectures are never added automatically to the database.
pub fn get_arches(c: &PgConnection) -> Result<Vec<Arch>, diesel::result::Error> {
    use crate::db::schema::arch::dsl::*;
    let query = arch.select((id, name));
    let debug = diesel::debug_query::<diesel::pg::Pg, _>(&query);
    print_step(debug.to_string());
    query.load::<Arch>(c)
}

/// Retrieve the existing versions from the database
///
/// If a new version if found that new version will be added to the database
pub fn get_versions(c: &PgConnection) -> Result<Vec<Version>, diesel::result::Error> {
    use crate::db::schema::version::dsl::*;
    let query = version.select((id, name, product_id, is_test));
    let debug = diesel::debug_query::<diesel::pg::Pg, _>(&query);
    print_step(debug.to_string());
    query.load::<Version>(c)
}

/// Creates a repository in the database using the given parameters.
pub fn create_repository(
    c: &PgConnection,
    directory_id: i32,
    with_topdir: String,
    cat_id: i32,
    version_id: i32,
    arch_id: i32,
    prefix: String,
) -> Result<usize, diesel::result::Error> {
    use crate::db::schema::repository;

    let insert = diesel::insert_into(repository::dsl::repository).values((
        repository::dsl::name.eq(with_topdir),
        repository::dsl::category_id.eq(cat_id),
        repository::dsl::version_id.eq(version_id),
        repository::dsl::arch_id.eq(arch_id),
        repository::dsl::directory_id.eq(directory_id),
        repository::dsl::prefix.eq(&prefix),
        repository::dsl::disabled.eq(false),
    ));

    STEPS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

    let debug = diesel::debug_query::<diesel::pg::Pg, _>(&insert);
    print_step(debug.to_string());
    println!(
        "Created Repository(prefix={}, version={}, arch={}, category={}) -> Directory {}",
        prefix, version_id, arch_id, cat_id, directory_id
    );
    insert.execute(c)
}

/// Get the table `file_detail` from the database
///
/// This data is used to store checksum about files. Needef
/// for metalinks. There can be multiple entries for a file.
///
/// This table is cleaned during `age_file_details()`.
pub fn get_file_details(c: &PgConnection) -> Vec<FileDetail> {
    use crate::db::schema::file_detail::dsl::*;
    let query = file_detail.select((
        id,
        directory_id,
        filename,
        timestamp,
        size,
        sha1,
        md5,
        sha256,
        sha512,
    ));
    let debug = diesel::debug_query::<diesel::pg::Pg, _>(&query);
    print_step(debug.to_string());
    query
        .load::<FileDetail>(c)
        .expect("Error loading file_detail")
}

/// Get list of categories.
///
/// Needed during startup to make sure the selected category
/// actually exists. It is also needed to figure out the topdir
/// for each category.
pub fn get_categories(c: &PgConnection) -> Vec<Category> {
    use crate::db::schema::category;
    use crate::db::schema::directory;

    let query = category::dsl::category
        .inner_join(directory::dsl::directory)
        .select((
            category::dsl::id,
            category::dsl::name,
            directory::dsl::name,
            category::dsl::product_id,
        ));
    let debug = diesel::debug_query::<diesel::pg::Pg, _>(&query);
    print_step(debug.to_string());
    query.load::<Category>(c).expect("Error loading categories")
}

/// Get all directories and the ctime.
///
/// ctime is the most important parameter to detect if
/// something has changed on the primary mirror.
pub fn get_directories(c: &PgConnection, cat_id: i32) -> Vec<Directory> {
    use crate::db::schema::category_directory;
    use crate::db::schema::directory;

    let subselect = category_directory::dsl::category_directory
        .select(category_directory::dsl::directory_id)
        .filter(category_directory::dsl::category_id.eq(cat_id));

    let query = directory::dsl::directory
        .select((
            directory::dsl::id,
            directory::dsl::name,
            directory::dsl::files,
            directory::dsl::readable,
            directory::dsl::ctime,
        ))
        .filter(directory::dsl::id.eq_any(subselect));
    let debug = diesel::debug_query::<diesel::pg::Pg, _>(&query);
    print_step(debug.to_string());
    query
        .load::<Directory>(c)
        .expect("Error loading directories")
}

/// This retrieves the list of which directory belongs to given category.
pub fn _get_category_directories(c: &PgConnection, cat_id: i32) -> Vec<CategoryDirectory> {
    use crate::db::schema::category_directory;

    let query = category_directory::dsl::category_directory
        .select((
            category_directory::dsl::category_id,
            category_directory::dsl::directory_id,
        ))
        .filter(category_directory::dsl::category_id.eq(cat_id));
    let debug = diesel::debug_query::<diesel::pg::Pg, _>(&query);
    print_step(debug.to_string());
    query
        .load::<CategoryDirectory>(c)
        .expect("Error loading category directories")
}

/// Get a list of all repositories MirrorManager knows about.
pub fn get_repositories(c: &PgConnection) -> Result<Vec<Repository>, diesel::result::Error> {
    use crate::db::schema::repository::dsl::*;

    let query = repository.select((
        id,
        name,
        prefix,
        category_id,
        version_id,
        arch_id,
        directory_id,
        disabled,
    ));
    let debug = diesel::debug_query::<diesel::pg::Pg, _>(&query);
    print_step(debug.to_string());
    query.load::<Repository>(c)
}
