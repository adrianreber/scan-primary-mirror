use crate::*;

#[test]
fn basename_test() {
    let mut path = String::from("onlybase");
    assert_eq!(basename(path), "onlybase");
    path = String::from("parent/base");
    assert_eq!(basename(path), "base");
    path = String::from("/noparent");
    assert_eq!(basename(path), "noparent");
    path = String::from("nobase/");
    assert_eq!(basename(path), "");
}

#[test]
fn ctime_from_rsync_test() {
    assert_eq!(-1, ctime_from_rsync("".to_string(), "".to_string()));
    assert_eq!(
        -1,
        ctime_from_rsync("test1".to_string(), "test2".to_string())
    );
    assert_eq!(
        -1,
        ctime_from_rsync("1234/56/76".to_string(), "12:34:56".to_string())
    );
    assert_eq!(
        -1,
        ctime_from_rsync("2000/00/00".to_string(), "12:34:56".to_string())
    );
    assert_eq!(
        946730096,
        ctime_from_rsync("2000/01/01".to_string(), "12:34:56".to_string())
    );
}

#[test]
fn get_version_from_path_test() {
    assert_eq!("", get_version_from_path("".to_string()));
    assert_eq!("", get_version_from_path("/".to_string()));
    assert_eq!("77", get_version_from_path("top/77/base".to_string()));
    assert_eq!("8.8", get_version_from_path("top/8.8/base".to_string()));
    assert_eq!("3", get_version_from_path("top/3/base".to_string()));
    assert_eq!(
        "development",
        get_version_from_path("top/development/rawhide/os".to_string())
    );
    assert_eq!("", get_version_from_path("top/development/os".to_string()));
    assert_eq!(
        "1030",
        get_version_from_path("top/development/1030/os".to_string())
    );
}

#[test]
fn repo_prefix_test() {
    let mut rms = vec![settings::RepositoryMapping {
        regex: "[(^^^^".to_string(),
        prefix: "some".to_string(),
    }];
    assert_eq!("", repo_prefix("path".to_string(), "76".to_string(), &rms));
    rms = vec![settings::RepositoryMapping {
        regex: "path".to_string(),
        prefix: "some".to_string(),
    }];
    assert_eq!(
        "some-76",
        repo_prefix("path".to_string(), "76".to_string(), &rms)
    );
    assert_eq!(
        "some-source-76",
        repo_prefix("path/SRPMS/debug".to_string(), "76".to_string(), &rms)
    );
    assert_eq!(
        "some-debug-76",
        repo_prefix("path/debug/os".to_string(), "76".to_string(), &rms)
    );
    rms = vec![
        settings::RepositoryMapping {
            regex: "^path/fedora/updates/[\\.\\d]+/.*".to_string(),
            prefix: "fedora-updates-released".to_string(),
        },
        settings::RepositoryMapping {
            regex: "^path/fedora/updates/testing/[\\.\\d]+/.*".to_string(),
            prefix: "fedora-updates-testing".to_string(),
        },
    ];
    assert_eq!(
        "",
        repo_prefix("path/debug/os".to_string(), "76".to_string(), &rms)
    );
    assert_eq!(
        "",
        repo_prefix("path/fedora/updates".to_string(), "76".to_string(), &rms)
    );
    assert_eq!(
        "fedora-updates-released-76",
        repo_prefix(
            "path/fedora/updates/76/".to_string(),
            "76".to_string(),
            &rms
        )
    );
    assert_eq!(
        "fedora-updates-testing-debug-76",
        repo_prefix(
            "path/fedora/updates/testing/76/debug".to_string(),
            "76".to_string(),
            &rms
        )
    );
}

#[test]
fn check_for_repo_test() {
    let repos = vec![db::models::Repository {
        id: 17,
        name: "repository/name/23".to_string(),
        prefix: Some("repository-name-23".to_string()),
        category_id: Some(6),
        version_id: Some(7),
        arch_id: Some(8),
        directory_id: Some(9),
        disabled: false,
    }];

    assert!(!check_for_repo(&repos, "prefix".to_string(), 4));
    assert!(!check_for_repo(&repos, "prefix".to_string(), 8));
    assert!(check_for_repo(&repos, "repository-name-23".to_string(), 8));
}

fn get_db_connection() -> Result<PgConnection, Box<dyn Error>> {
    let database_url = env::var("TEST_DATABASE_URL")?;

    Ok(PgConnection::establish(&database_url)?)
}

#[test]
fn age_file_details_test() {
    let c = match get_db_connection() {
        Ok(c) => c,
        Err(e) => {
            println!("Database connection failed {}", e);
            assert!(false);
            return;
        }
    };

    use db::schema::file_detail;
    assert!(!diesel::delete(db::schema::file_detail::dsl::file_detail)
        .execute(&c)
        .is_err());

    #[derive(Insertable, Debug)]
    #[table_name = "file_detail"]
    struct InsertFileDetail {
        directory_id: i32,
        filename: String,
        timestamp: Option<i64>,
        size: Option<i64>,
        sha1: Option<String>,
        md5: Option<String>,
        sha256: Option<String>,
        sha512: Option<String>,
    }

    let five = chrono::offset::Local::now().timestamp() - (60 * 60 * 24 * 5);
    let four = chrono::offset::Local::now().timestamp() - (60 * 60 * 24 * 4);
    let three = chrono::offset::Local::now().timestamp() - (60 * 60 * 24 * 3);

    let mut ifds: Vec<InsertFileDetail> = Vec::new();

    let fd_7_5 = InsertFileDetail {
        directory_id: 7,
        filename: String::from("repomd.xml"),
        timestamp: Some(five),
        size: Some(40),
        sha1: Some(String::from("sha1")),
        md5: Some(String::from("md5")),
        sha256: Some(String::from("sha256")),
        sha512: Some(String::from("sha512")),
    };
    let fd_8_3 = InsertFileDetail {
        directory_id: 8,
        filename: String::from("repomd.xml"),
        timestamp: Some(three),
        size: Some(40),
        sha1: Some(String::from("sha1")),
        md5: Some(String::from("md5")),
        sha256: Some(String::from("sha256")),
        sha512: Some(String::from("sha512")),
    };
    let fd_7_4 = InsertFileDetail {
        directory_id: 7,
        filename: String::from("repomd.xml"),
        timestamp: Some(four),
        size: Some(40),
        sha1: Some(String::from("sha1")),
        md5: Some(String::from("md5")),
        sha256: Some(String::from("sha256")),
        sha512: Some(String::from("sha512")),
    };
    let fd_7_3 = InsertFileDetail {
        directory_id: 7,
        filename: String::from("repomd.xml"),
        timestamp: Some(three),
        size: Some(40),
        sha1: Some(String::from("sha1")),
        md5: Some(String::from("md5")),
        sha256: Some(String::from("sha256")),
        sha512: Some(String::from("sha512")),
    };

    ifds.push(fd_7_5);
    ifds.push(fd_8_3);
    ifds.push(fd_7_4);
    ifds.push(fd_7_3);

    if let Err(e) = diesel::insert_into(db::schema::file_detail::dsl::file_detail)
        .values(&ifds)
        .execute(&c)
    {
        println!("Database insert failed {}", e);
        assert!(false);
    }

    let mut fds = db::functions::get_file_details(&c);
    let fds_org = db::functions::get_file_details(&c);
    if let Err(e) = age_file_details(&c, &mut fds, 6, 5) {
        println!("Running age_file_details() failed {}", e);
        assert!(false);
    }
    assert!(fds_org
        .iter()
        .eq(db::functions::get_file_details(&c).iter()));

    fds = db::functions::get_file_details(&c);
    if let Err(e) = age_file_details(&c, &mut fds, 4, 3) {
        println!("Running age_file_details() failed {}", e);
        assert!(false);
    }
    assert_eq!(3, db::functions::get_file_details(&c).len());

    fds = db::functions::get_file_details(&c);
    if let Err(e) = age_file_details(&c, &mut fds, 1, 0) {
        println!("Running age_file_details() failed {}", e);
        assert!(false);
    }
    assert_eq!(2, db::functions::get_file_details(&c).len());
}

#[test]
fn sync_category_directories_test() {
    let c = match get_db_connection() {
        Ok(c) => c,
        Err(e) => {
            println!("Database connection failed {}", e);
            assert!(false);
            return;
        }
    };

    // clean tables for test
    assert!(
        !diesel::delete(db::schema::category_directory::dsl::category_directory)
            .execute(&c)
            .is_err()
    );
    assert!(!diesel::delete(db::schema::file_detail::dsl::file_detail)
        .execute(&c)
        .is_err());
    assert!(!diesel::delete(db::schema::directory::dsl::directory)
        .execute(&c)
        .is_err());

    // this is empty
    let mut dirs = db::functions::get_directories(&c);
    let mut cds: HashMap<String, CategoryDirectory> = HashMap::new();

    let mut cd1 = CategoryDirectory {
        files: Vec::new(),
        ctime: 1000,
        readable: true,
        directory_id: 7,
        ctime_changed: true,
    };

    cds.insert("directory1".to_string(), cd1.clone());

    assert!(
        !sync_category_directories(&c, "topdir/".to_string(), 37, &mut dirs, &mut cds).is_err()
    );
    // now it should contain the entry from above
    assert_eq!(dirs.len(), 1);
    assert_eq!(dirs[0].ctime, 1000);
    assert_eq!(dirs[0].readable, true);
    assert_eq!(dirs[0].name, "topdir/directory1".to_string());

    dirs = db::functions::get_directories(&c);
    // test after reading from database
    assert_eq!(dirs.len(), 1);
    assert_eq!(dirs[0].ctime, 1000);
    assert_eq!(dirs[0].readable, true);
    assert_eq!(dirs[0].name, "topdir/directory1".to_string());
    // update entry
    cd1.ctime = 2000;
    cds = HashMap::new();
    cds.insert("directory1".to_string(), cd1);
    assert!(
        !sync_category_directories(&c, "topdir/".to_string(), 37, &mut dirs, &mut cds).is_err()
    );
    dirs = db::functions::get_directories(&c);
    assert_eq!(dirs.len(), 1);
    // this should have been updated
    assert_eq!(dirs[0].ctime, 2000);
    assert_eq!(dirs[0].readable, true);
    assert_eq!(dirs[0].name, "topdir/directory1".to_string());
}

#[test]
fn is_excluded_test() {
    assert!(is_excluded("path".to_string(), &["[p]".to_string()]));
    assert!(!is_excluded("path".to_string(), &["[o]".to_string()]));
    assert!(is_excluded(
        "path".to_string(),
        &["[o]".to_string(), "[p]".to_string()]
    ));
    assert!(is_excluded(
        "topdir/.snapshot/directory1".to_string(),
        &[
            "pattern1".to_string(),
            "[p".to_string(),
            ".*\\.snapshot".to_string()
        ]
    ));
}

fn get_insert_versions(
    c: &PgConnection,
) -> Result<Vec<db::models::InsertVersion>, diesel::result::Error> {
    use crate::db::schema::version::dsl::*;
    let query = version.select((
        id,
        name,
        product_id,
        is_test,
        sortorder,
        display,
        ordered_mirrorlist,
    ));
    query.load::<db::models::InsertVersion>(c)
}

#[test]
fn guess_ver_arch_from_path_test() {
    let c = match get_db_connection() {
        Ok(c) => c,
        Err(e) => {
            println!("Database connection failed {}", e);
            assert!(false);
            return;
        }
    };

    // clean tables for test
    assert!(!diesel::delete(db::schema::version::dsl::version)
        .execute(&c)
        .is_err());

    let arches = vec![db::models::Arch {
        id: 43,
        name: "unexp".to_string(),
    }];

    let mut versions: Vec<db::models::Version> = Vec::new();
    let mut test_paths: Vec<String> = Vec::new();
    let mut do_not_display_paths: Vec<String> = Vec::new();

    match guess_ver_arch_from_path(
        &c,
        "path/with/no/version".to_string(),
        &arches,
        &mut versions,
        87,
        &test_paths,
        &do_not_display_paths,
    ) {
        Ok(_) => assert!(false),
        Err(e) => assert_eq!(format!("{}", e), "Not able to figure out architecture"),
    };

    let mut result = match guess_ver_arch_from_path(
        &c,
        "path/with/unexp/version".to_string(),
        &arches,
        &mut versions,
        87,
        &test_paths,
        &do_not_display_paths,
    ) {
        Ok(r) => r,
        Err(e) => {
            println!("{}", e);
            assert!(false);
            ("".to_string(), -1, -1)
        }
    };

    assert_eq!(-1, result.1);
    assert_eq!(43, result.2);

    result = match guess_ver_arch_from_path(
        &c,
        "path/with/unexp/8.88/something".to_string(),
        &arches,
        &mut versions,
        87,
        &test_paths,
        &do_not_display_paths,
    ) {
        Ok(r) => r,
        Err(e) => {
            println!("{}", e);
            assert!(false);
            ("".to_string(), -1, -1)
        }
    };

    assert_eq!("8.88", result.0);
    assert_eq!(43, result.2);
    versions = match db::functions::get_versions(&c) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", e);
            assert!(false);
            Vec::new()
        }
    };
    assert_eq!(1, versions.len());
    assert_eq!("8.88", versions[0].name);
    assert_eq!(false, versions[0].is_test);
    assert_eq!(87, versions[0].product_id);

    // clean tables for test
    assert!(!diesel::delete(db::schema::version::dsl::version)
        .execute(&c)
        .is_err());
    versions = Vec::new();

    test_paths.push("/with/".to_string());
    result = match guess_ver_arch_from_path(
        &c,
        "path/with/unexp/8.88/something".to_string(),
        &arches,
        &mut versions,
        87,
        &test_paths,
        &do_not_display_paths,
    ) {
        Ok(r) => r,
        Err(e) => {
            println!("{}", e);
            assert!(false);
            ("".to_string(), -1, -1)
        }
    };

    assert_eq!("8.88", result.0);
    assert_eq!(43, result.2);

    let mut insert_versions = match get_insert_versions(&c) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", e);
            assert!(false);
            Vec::new()
        }
    };
    assert_eq!(1, insert_versions.len());
    assert_eq!("8.88", insert_versions[0].name);
    assert_eq!(true, insert_versions[0].is_test);
    assert_eq!(true, insert_versions[0].display);
    assert_eq!(87, insert_versions[0].product_id);

    // clean tables for test
    assert!(!diesel::delete(db::schema::version::dsl::version)
        .execute(&c)
        .is_err());
    versions = Vec::new();
    do_not_display_paths = vec!["_Beta".to_string()];

    test_paths.push("/with/".to_string());
    result = match guess_ver_arch_from_path(
        &c,
        "path/with/unexp/8.88_Beta/something".to_string(),
        &arches,
        &mut versions,
        87,
        &test_paths,
        &do_not_display_paths,
    ) {
        Ok(r) => r,
        Err(e) => {
            println!("{}", e);
            assert!(false);
            ("".to_string(), -1, -1)
        }
    };

    assert_eq!("8.88_Beta", result.0);
    assert_eq!(43, result.2);

    insert_versions = match get_insert_versions(&c) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", e);
            assert!(false);
            Vec::new()
        }
    };
    assert_eq!(1, insert_versions.len());
    assert_eq!("8.88_Beta", insert_versions[0].name);
    assert_eq!(true, insert_versions[0].is_test);
    assert_eq!(false, insert_versions[0].display);
    assert_eq!(87, insert_versions[0].product_id);
}
