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
    assert_eq!(
        "9-stream",
        get_version_from_path("SIGs/9-stream/infra/x86_64/infra-common/".to_string())
    );
}

#[test]
fn repo_prefix_test() {
    let mut rms = vec![settings::RepositoryMapping {
        regex: "[(^^^^".to_string(),
        prefix: "some".to_string(),
        version_prefix: None,
    }];
    let mut aliases = vec![settings::RepositoryAlias {
        from: "testing-modular-epel-debug-".to_string(),
        to: "testing-modular-debug-epel".to_string(),
    }];
    assert_eq!(
        "",
        repo_prefix("path".to_string(), "76".to_string(), &rms, &aliases)
    );
    rms = vec![settings::RepositoryMapping {
        regex: "path".to_string(),
        prefix: "some".to_string(),
        version_prefix: None,
    }];
    assert_eq!(
        "some-76",
        repo_prefix("path".to_string(), "76".to_string(), &rms, &aliases)
    );
    assert_eq!(
        "some-source-76",
        repo_prefix(
            "path/SRPMS/debug".to_string(),
            "76".to_string(),
            &rms,
            &aliases,
        )
    );
    assert_eq!(
        "some-source-76",
        repo_prefix(
            "path/source/repodata".to_string(),
            "76".to_string(),
            &rms,
            &aliases,
        )
    );
    assert_eq!(
        "some-source-76",
        repo_prefix(
            "path/src/repodata".to_string(),
            "76".to_string(),
            &rms,
            &aliases,
        )
    );
    assert_eq!(
        "some-debug-76",
        repo_prefix(
            "path/debug/os".to_string(),
            "76".to_string(),
            &rms,
            &aliases,
        )
    );
    rms = vec![settings::RepositoryMapping {
        regex: "path".to_string(),
        prefix: "some".to_string(),
        version_prefix: Some("f".to_string()),
    }];
    assert_eq!(
        "some-f76",
        repo_prefix("path".to_string(), "76".to_string(), &rms, &aliases,)
    );
    rms = vec![
        settings::RepositoryMapping {
            regex: "^path/fedora/updates/[\\.\\d]+/.*".to_string(),
            prefix: "fedora-updates-released".to_string(),
            version_prefix: None,
        },
        settings::RepositoryMapping {
            regex: "^path/fedora/updates/testing/[\\.\\d]+/.*".to_string(),
            prefix: "fedora-updates-testing".to_string(),
            version_prefix: None,
        },
        settings::RepositoryMapping {
            regex:
                "^SIGs/\\d+(?:-stream)?/(?P<signame>\\S+?)/(?P<arch>\\S+?)/(?P<sigrepo>\\S+?)/.*"
                    .to_string(),
            prefix: "centos-${signame}-sig-${sigrepo}".to_string(),
            version_prefix: None,
        },
    ];
    assert_eq!(
        "centos-infra-sig-infra-common-9-stream",
        repo_prefix(
            "SIGs/9-stream/infra/x86_64/infra-common/repodata".to_string(),
            "9-stream".to_string(),
            &rms,
            &aliases,
        )
    );
    assert_eq!(
        "centos-infra-sig-infra-common-debug-9-stream",
        repo_prefix(
            "SIGs/9-stream/infra/x86_64/infra-common/debug/repodata".to_string(),
            "9-stream".to_string(),
            &rms,
            &aliases,
        )
    );
    assert_eq!(
        "centos-infra-sig-infra-common-source-9-stream",
        repo_prefix(
            "SIGs/9-stream/infra/source/infra-common/Packages".to_string(),
            "9-stream".to_string(),
            &rms,
            &aliases,
        )
    );
    assert_eq!(
        "",
        repo_prefix(
            "path/debug/os".to_string(),
            "76".to_string(),
            &rms,
            &aliases,
        )
    );
    assert_eq!(
        "",
        repo_prefix(
            "path/fedora/updates".to_string(),
            "76".to_string(),
            &rms,
            &aliases,
        )
    );
    assert_eq!(
        "fedora-updates-released-76",
        repo_prefix(
            "path/fedora/updates/76/".to_string(),
            "76".to_string(),
            &rms,
            &aliases,
        )
    );
    assert_eq!(
        "fedora-updates-testing-debug-76",
        repo_prefix(
            "path/fedora/updates/testing/76/debug".to_string(),
            "76".to_string(),
            &rms,
            &aliases,
        )
    );
    aliases = vec![settings::RepositoryAlias {
        from: "fedora-updates-testing-debug-".to_string(),
        to: "debug-testing-updates-fedora".to_string(),
    }];
    assert_eq!(
        "debug-testing-updates-fedora76",
        repo_prefix(
            "path/fedora/updates/testing/76/debug".to_string(),
            "76".to_string(),
            &rms,
            &aliases,
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
    let mut c = match get_db_connection() {
        Ok(c) => c,
        Err(e) => {
            println!("Database connection failed {}", e);
            panic!();
        }
    };

    let dirs = vec![
        db::models::Directory {
            id: 7,
            name: "dirname7".to_string(),
            files: Vec::new(),
            readable: true,
            ctime: 0,
        },
        db::models::Directory {
            id: 8,
            name: "dirname8".to_string(),
            files: Vec::new(),
            readable: true,
            ctime: 0,
        },
        db::models::Directory {
            id: 9,
            name: "dirname9".to_string(),
            files: Vec::new(),
            readable: true,
            ctime: 0,
        },
    ];

    assert!(diesel::delete(db::schema::file_detail::dsl::file_detail)
        .execute(&mut c)
        .is_ok());

    let five = chrono::offset::Local::now().timestamp() - (60 * 60 * 24 * 5) + 1000;
    let four = chrono::offset::Local::now().timestamp() - (60 * 60 * 24 * 4) + 1000;
    let three = chrono::offset::Local::now().timestamp() - (60 * 60 * 24 * 3) + 1000;

    let mut ifds: Vec<db::models::InsertFileDetail> = Vec::new();

    let fd_7_5 = db::models::InsertFileDetail {
        directory_id: 7,
        filename: String::from("repomd.xml"),
        timestamp: Some(five),
        size: Some(40),
        sha1: Some(String::from("sha1")),
        md5: Some(String::from("md5")),
        sha256: Some(String::from("sha256")),
        sha512: Some(String::from("sha512")),
    };
    let fd_7_5_other = db::models::InsertFileDetail {
        directory_id: 7,
        filename: String::from("other_name"),
        timestamp: Some(five),
        size: Some(40),
        sha1: Some(String::from("sha1")),
        md5: Some(String::from("md5")),
        sha256: Some(String::from("sha256")),
        sha512: Some(String::from("sha512")),
    };
    let fd_8_3 = db::models::InsertFileDetail {
        directory_id: 8,
        filename: String::from("repomd.xml"),
        timestamp: Some(three),
        size: Some(40),
        sha1: Some(String::from("sha1")),
        md5: Some(String::from("md5")),
        sha256: Some(String::from("sha256")),
        sha512: Some(String::from("sha512")),
    };
    let fd_7_4 = db::models::InsertFileDetail {
        directory_id: 7,
        filename: String::from("repomd.xml"),
        timestamp: Some(four),
        size: Some(40),
        sha1: Some(String::from("sha1")),
        md5: Some(String::from("md5")),
        sha256: Some(String::from("sha256")),
        sha512: Some(String::from("sha512")),
    };
    let fd_7_3 = db::models::InsertFileDetail {
        directory_id: 7,
        filename: String::from("repomd.xml"),
        timestamp: Some(three),
        size: Some(40),
        sha1: Some(String::from("sha1")),
        md5: Some(String::from("md5")),
        sha256: Some(String::from("sha256")),
        sha512: Some(String::from("sha512")),
    };
    let fd_10_5 = db::models::InsertFileDetail {
        directory_id: 10,
        filename: String::from("repomd.xml"),
        timestamp: Some(five),
        size: Some(40),
        sha1: Some(String::from("sha1")),
        md5: Some(String::from("md5")),
        sha256: Some(String::from("sha256")),
        sha512: Some(String::from("sha512")),
    };
    let fd_10_4 = db::models::InsertFileDetail {
        directory_id: 10,
        filename: String::from("repomd.xml"),
        timestamp: Some(four),
        size: Some(40),
        sha1: Some(String::from("sha1")),
        md5: Some(String::from("md5")),
        sha256: Some(String::from("sha256")),
        sha512: Some(String::from("sha512")),
    };
    let fd_10_3 = db::models::InsertFileDetail {
        directory_id: 10,
        filename: String::from("repomd.xml"),
        timestamp: Some(three),
        size: Some(40),
        sha1: Some(String::from("sha1")),
        md5: Some(String::from("md5")),
        sha256: Some(String::from("sha256")),
        sha512: Some(String::from("sha512")),
    };

    ifds.push(fd_7_5);
    ifds.push(fd_7_5_other);
    ifds.push(fd_8_3);
    ifds.push(fd_7_4);
    ifds.push(fd_7_3);
    ifds.push(fd_10_5);
    ifds.push(fd_10_4);
    ifds.push(fd_10_3);

    if let Err(e) = diesel::insert_into(db::schema::file_detail::dsl::file_detail)
        .values(&ifds)
        .execute(&mut c)
    {
        println!("Database insert failed {}", e);
        panic!();
    }

    let mut fds = db::functions::get_file_details(&mut c);
    let fds_org = db::functions::get_file_details(&mut c);
    if let Err(e) = age_file_details(&mut c, &mut fds, &dirs, 6, 5) {
        println!("Running age_file_details() failed {}", e);
        panic!();
    }
    assert!(fds_org
        .iter()
        .eq(db::functions::get_file_details(&mut c).iter()));

    fds = db::functions::get_file_details(&mut c);
    if let Err(e) = age_file_details(&mut c, &mut fds, &dirs, 4, 3) {
        println!("Running age_file_details() failed {}", e);
        panic!();
    }
    assert_eq!(7, db::functions::get_file_details(&mut c).len());

    fds = db::functions::get_file_details(&mut c);
    if let Err(e) = age_file_details(&mut c, &mut fds, &dirs, 1, 0) {
        println!("Running age_file_details() failed {}", e);
        panic!();
    }
    assert_eq!(6, db::functions::get_file_details(&mut c).len());
}

#[test]
fn sync_category_directories_test() {
    let mut c = match get_db_connection() {
        Ok(c) => c,
        Err(e) => {
            println!("Database connection failed {}", e);
            panic!();
        }
    };

    // clean tables for test
    assert!(
        diesel::delete(db::schema::category_directory::dsl::category_directory)
            .execute(&mut c)
            .is_ok()
    );
    assert!(diesel::delete(db::schema::file_detail::dsl::file_detail)
        .execute(&mut c)
        .is_ok());
    assert!(diesel::delete(db::schema::directory::dsl::directory)
        .execute(&mut c)
        .is_ok());

    // this is empty
    let mut dirs = db::functions::get_directories(&mut c, 37);
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
        sync_category_directories(&mut c, "topdir/".to_string(), 37, &mut dirs, &mut cds).is_ok()
    );
    // now it should contain the entry from above
    assert_eq!(dirs.len(), 1);
    assert_eq!(dirs[0].ctime, 1000);
    assert!(dirs[0].readable);
    assert_eq!(dirs[0].name, "topdir/directory1".to_string());

    dirs = db::functions::get_directories(&mut c, 37);
    // test after reading from database
    assert_eq!(dirs.len(), 1);
    assert_eq!(dirs[0].ctime, 1000);
    assert!(dirs[0].readable);
    assert_eq!(dirs[0].name, "topdir/directory1".to_string());
    // update entry
    cd1.ctime = 2000;
    cds = HashMap::new();
    cds.insert("directory1".to_string(), cd1);
    assert!(
        sync_category_directories(&mut c, "topdir/".to_string(), 37, &mut dirs, &mut cds).is_ok()
    );
    dirs = db::functions::get_directories(&mut c, 37);
    assert_eq!(dirs.len(), 1);
    // this should have been updated
    assert_eq!(dirs[0].ctime, 2000);
    assert!(dirs[0].readable);
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
    c: &mut PgConnection,
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
    let mut c = match get_db_connection() {
        Ok(c) => c,
        Err(e) => {
            println!("Database connection failed {}", e);
            panic!();
        }
    };

    // clean tables for test
    assert!(diesel::delete(db::schema::version::dsl::version)
        .execute(&mut c)
        .is_ok());

    let arches = vec![db::models::Arch {
        id: 43,
        name: "unexp".to_string(),
    }];

    let mut versions: Vec<db::models::Version> = Vec::new();
    let mut test_paths: Vec<String> = Vec::new();
    let mut do_not_display_paths: Vec<String> = Vec::new();

    match guess_ver_arch_from_path(
        &mut c,
        "path/with/no/version".to_string(),
        &arches,
        &mut versions,
        87,
        &test_paths,
        &do_not_display_paths,
    ) {
        Ok(_) => panic!(),
        Err(e) => assert_eq!(format!("{}", e), "Not able to figure out architecture"),
    };

    let mut result = match guess_ver_arch_from_path(
        &mut c,
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
            panic!();
        }
    };

    assert_eq!(-1, result.1);
    assert_eq!(43, result.2);

    result = match guess_ver_arch_from_path(
        &mut c,
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
            panic!();
        }
    };

    assert_eq!("8.88", result.0);
    assert_eq!(43, result.2);
    versions = match db::functions::get_versions(&mut c) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", e);
            panic!();
        }
    };
    assert_eq!(1, versions.len());
    assert_eq!("8.88", versions[0].name);
    assert!(!versions[0].is_test);
    assert_eq!(87, versions[0].product_id);

    // clean tables for test
    assert!(diesel::delete(db::schema::version::dsl::version)
        .execute(&mut c)
        .is_ok());
    versions = Vec::new();

    test_paths.push("/with/".to_string());
    result = match guess_ver_arch_from_path(
        &mut c,
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
            panic!();
        }
    };

    assert_eq!("8.88", result.0);
    assert_eq!(43, result.2);

    let mut insert_versions = match get_insert_versions(&mut c) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", e);
            panic!();
        }
    };
    assert_eq!(1, insert_versions.len());
    assert_eq!("8.88", insert_versions[0].name);
    assert!(insert_versions[0].is_test);
    assert!(insert_versions[0].display);
    assert_eq!(87, insert_versions[0].product_id);

    // clean tables for test
    assert!(diesel::delete(db::schema::version::dsl::version)
        .execute(&mut c)
        .is_ok());
    versions = Vec::new();
    do_not_display_paths = vec!["_Beta".to_string()];

    test_paths.push("/with/".to_string());
    result = match guess_ver_arch_from_path(
        &mut c,
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
            panic!();
        }
    };

    assert_eq!("8.88_Beta", result.0);
    assert_eq!(43, result.2);

    insert_versions = match get_insert_versions(&mut c) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", e);
            panic!();
        }
    };
    assert_eq!(1, insert_versions.len());
    assert_eq!("8.88_Beta", insert_versions[0].name);
    assert!(insert_versions[0].is_test);
    assert!(!insert_versions[0].display);
    assert_eq!(87, insert_versions[0].product_id);
}

#[test]
fn guess_ver_arch_from_path_test_with_rawhide() {
    let mut c = match get_db_connection() {
        Ok(c) => c,
        Err(e) => {
            println!("Database connection failed {}", e);
            panic!();
        }
    };

    // clean tables for test
    assert!(diesel::delete(db::schema::version::dsl::version)
        .execute(&mut c)
        .is_ok());

    let arches = vec![db::models::Arch {
        id: 43,
        name: "unexp".to_string(),
    }];

    let mut versions = vec![db::models::Version {
        id: 783,
        name: "development".to_string(),
        product_id: 87,
        is_test: false,
    }];
    let test_paths: Vec<String> = Vec::new();
    let do_not_display_paths: Vec<String> = Vec::new();

    let mut result = match guess_ver_arch_from_path(
        &mut c,
        "path/development/unexp/rawhide/something".to_string(),
        &arches,
        &mut versions,
        87,
        &test_paths,
        &do_not_display_paths,
    ) {
        Ok(r) => r,
        Err(e) => {
            println!("{}", e);
            panic!();
        }
    };

    assert_eq!("rawhide".to_string(), result.0);
    assert_eq!(43, result.2);

    result = match guess_ver_arch_from_path(
        &mut c,
        "path/development/93/unexp/something".to_string(),
        &arches,
        &mut versions,
        87,
        &test_paths,
        &do_not_display_paths,
    ) {
        Ok(r) => r,
        Err(e) => {
            println!("{}", e);
            panic!();
        }
    };

    assert_eq!("93".to_string(), result.0);
    assert_eq!(43, result.2);

    // clean tables after test
    assert!(diesel::delete(db::schema::version::dsl::version)
        .execute(&mut c)
        .is_ok());
}

#[test]
fn get_timestamp_test() {
    let mut xml1 = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
    xml1.push_str("<repomd>");
    xml1.push_str("<revision>8.3</revision>");
    let mut ts = xml::get_timestamp(xml1.clone());
    assert_eq!(0, ts);

    let mut xml2 = xml1.clone();
    xml2.push_str("<data><timestamp>7</timestamp></data>");
    ts = xml::get_timestamp(xml2.clone());
    assert_eq!(0, ts);

    xml2.push_str("</repomd>");
    ts = xml::get_timestamp(xml2.clone());
    assert_eq!(7, ts);

    xml2 = xml1.clone();
    xml2.push_str("<data><timestamp>r</timestamp></data>");
    xml2.push_str("</repomd>");
    ts = xml::get_timestamp(xml2.clone());
    assert_eq!(-1, ts);

    xml2 = xml1.clone();
    xml2.push_str("<data><timestamp>7.r</timestamp></data>");
    xml2.push_str("</repomd>");
    ts = xml::get_timestamp(xml2.clone());
    assert_eq!(-1, ts);

    xml2 = xml1.clone();
    xml2.push_str("<data><timestamp>7.9</timestamp></data>");
    xml2.push_str("</repomd>");
    ts = xml::get_timestamp(xml2);
    assert_eq!(7, ts);

    xml1.push_str("<data><timestamp>7.9</timestamp></data>");
    xml1.push_str("<data><timestamp>9</timestamp></data>");
    xml1.push_str("<data><timestamp>-1</timestamp></data>");
    xml1.push_str("<data><timestamp>3</timestamp></data>");
    xml1.push_str("</repomd>");
    ts = xml::get_timestamp(xml1);
    assert_eq!(9, ts);
}

#[test]
fn get_details_test() {
    assert!(get_details(&None, "test", "", "repomd.xml", "rsync").is_err());
    println!(
        "{:#?}",
        get_details(
            &Some("http://www.example/".to_string()),
            "test",
            "",
            "repomd.xml",
            "rsync"
        )
    );
    assert!(get_details(
        &Some("http://www.example/".to_string()),
        "test",
        "",
        "repomd.xml",
        "rsync"
    )
    .is_err());

    let mut drs = match get_details(
        &Some("http://localhost:17397/".to_string()),
        "test",
        "",
        "repomd.xml",
        "rsync",
    ) {
        Ok(d) => d,
        Err(e) => {
            println!("Error {}", e);
            panic!();
        }
    };
    assert_eq!(drs[0].md5_sum, "c3002eefddc963954306e38632dbeca4");
    assert_eq!(drs[0].sha1_sum, "226c58402e3bc46d4e9f8bde740594430f4eea01");
    assert_eq!(
        drs[0].sha256_sum,
        "fa079bc0df97e4479c950b64e1f34c74a9da393f80eba7218c56edf8931907ce"
    );
    assert_eq!(drs[0].sha512_sum, "bccb1871d3b5670fdc6967155bced4d4f9978f4428c19407733bfabbf7aecb6c9de566629fed5e0d41c3ab93c96562dfc49e5ebb6a8f8dffcbbc411b5d1d9d52");
    assert_eq!(drs[0].length, 93);
    assert_eq!(drs[0].timestamp, 7);
    assert_eq!(drs[0].target, "repomd.xml");

    // Same test using 'directory' backend
    drs = match get_details(&Some("".to_string()), "test", "", "repomd.xml", "directory") {
        Ok(d) => d,
        Err(e) => {
            println!("Error {}", e);
            panic!();
        }
    };
    assert_eq!(drs[0].md5_sum, "c3002eefddc963954306e38632dbeca4");
    assert_eq!(drs[0].sha1_sum, "226c58402e3bc46d4e9f8bde740594430f4eea01");
    assert_eq!(
        drs[0].sha256_sum,
        "fa079bc0df97e4479c950b64e1f34c74a9da393f80eba7218c56edf8931907ce"
    );
    assert_eq!(drs[0].sha512_sum, "bccb1871d3b5670fdc6967155bced4d4f9978f4428c19407733bfabbf7aecb6c9de566629fed5e0d41c3ab93c96562dfc49e5ebb6a8f8dffcbbc411b5d1d9d52");
    assert_eq!(drs[0].length, 93);
    assert_eq!(drs[0].timestamp, 7);
    assert_eq!(drs[0].target, "repomd.xml");
}

#[test]
fn fill_ifds_test() {
    let mut ifds: Vec<db::models::InsertFileDetail> = Vec::new();
    let mut fds: Vec<db::models::FileDetail> = Vec::new();

    if fill_ifds(&mut FillIfds {
        ifds: &mut ifds,
        target: "repomd.xml",
        backend: "never-supported",
        checksum_base: &Some("http://localhost:17397/".to_string()),
        topdir: "test",
        dir: "",
        d_id: 65,
        fds: &fds,
        files: &None,
    })
    .is_ok()
    {
        panic!();
    }
    if fill_ifds(&mut FillIfds {
        ifds: &mut ifds,
        target: "repomd.xml",
        backend: "rsync",
        checksum_base: &Some("http://localhost:17397/".to_string()),
        topdir: "test",
        dir: "",
        d_id: 65,
        fds: &fds,
        files: &None,
    })
    .is_err()
    {
        panic!();
    }

    assert_eq!(ifds.len(), 1);
    assert_eq!(ifds[0].directory_id, 65);
    assert_eq!(ifds[0].filename, "repomd.xml".to_string());
    assert_eq!(ifds[0].timestamp, Some(7));
    assert_eq!(ifds[0].size, Some(93));
    assert_eq!(
        ifds[0].sha1,
        Some("226c58402e3bc46d4e9f8bde740594430f4eea01".to_string())
    );
    assert_eq!(
        ifds[0].md5,
        Some("c3002eefddc963954306e38632dbeca4".to_string())
    );
    assert_eq!(
        ifds[0].sha256,
        Some("fa079bc0df97e4479c950b64e1f34c74a9da393f80eba7218c56edf8931907ce".to_string())
    );
    assert_eq!(ifds[0].sha512, Some("bccb1871d3b5670fdc6967155bced4d4f9978f4428c19407733bfabbf7aecb6c9de566629fed5e0d41c3ab93c96562dfc49e5ebb6a8f8dffcbbc411b5d1d9d52".to_string()));

    ifds = Vec::new();
    fds = vec![ db::models::FileDetail {
        id: 39,
        directory_id: 65,
        filename: "repomd.xml".to_string(),
        timestamp: Some(7),
        size: Some(93),
        sha1: Some("226c58402e3bc46d4e9f8bde740594430f4eea01".to_string()),
        md5: Some("c3002eefddc963954306e38632dbeca4".to_string()),
        sha256: Some("fa079bc0df97e4479c950b64e1f34c74a9da393f80eba7218c56edf8931907ce".to_string()),
        sha512: Some("bccb1871d3b5670fdc6967155bced4d4f9978f4428c19407733bfabbf7aecb6c9de566629fed5e0d41c3ab93c96562dfc49e5ebb6a8f8dffcbbc411b5d1d9d52".to_string()),
    } ];
    if fill_ifds(&mut FillIfds {
        ifds: &mut ifds,
        target: "repomd.xml",
        backend: "rsync",
        checksum_base: &Some("http://localhost:17397/".to_string()),
        topdir: "test",
        dir: "",
        d_id: 65,
        fds: &fds,
        files: &None,
    })
    .is_err()
    {
        panic!();
    }

    assert_eq!(ifds.len(), 0);

    fds[0].timestamp = Some(6);
    if fill_ifds(&mut FillIfds {
        ifds: &mut ifds,
        target: "repomd.xml",
        backend: "rsync",
        checksum_base: &Some("http://localhost:17397/".to_string()),
        topdir: "test",
        dir: "",
        d_id: 65,
        fds: &fds,
        files: &None,
    })
    .is_err()
    {
        panic!();
    }

    assert_eq!(ifds.len(), 1);
}

#[test]
fn scan_with_rsync_test() {
    let mut cds: HashMap<String, CategoryDirectory> = HashMap::new();

    if scan_with_rsync(&mut cds, &[], "topdir/", &[], &[], "/this/should/not/exist").is_err() {
        panic!();
    }
    assert_eq!(cds.len(), 0);
    if scan_with_rsync(&mut cds, &[], "topdir/", &[], &[], "test").is_err() {
        panic!();
    }
    assert_eq!(cds.len(), 1);

    let mut repomd_found = false;

    for f in cds["test"].files.clone() {
        if f.name == "repomd.xml" && f.size == 93 {
            repomd_found = true;
        }
    }

    assert!(repomd_found);

    if scan_with_rsync(&mut cds, &[], "", &[], &[], "test").is_err() {
        panic!();
    }
    assert_eq!(cds.len(), 1);

    repomd_found = false;

    for f in cds["test"].files.clone() {
        if f.name == "repomd.xml" && f.size == 93 {
            repomd_found = true;
        }
    }

    assert!(repomd_found);
}

#[test]
fn scan_local_directory_test() {
    let mut cds: HashMap<String, CategoryDirectory> = HashMap::new();

    if scan_local_directory(
        &mut cds,
        &[],
        "topdir/",
        "/this/should/not/exist",
        false,
        "Test Category",
    )
    .is_err()
    {
        panic!();
    }
    assert_eq!(cds.len(), 0);
    if scan_local_directory(&mut cds, &[], "es", "test", false, "Test Category").is_err() {
        panic!();
    }
    println!("{:#?}", cds);
    assert_eq!(cds.len(), 1);

    let mut repomd_found = false;

    for f in cds["t"].files.clone() {
        if f.name == "repomd.xml" && f.size == 93 {
            repomd_found = true;
        }
    }

    assert!(repomd_found);

    repomd_found = false;
    cds = HashMap::new();
    let mut sql_found = false;
    let content = format!(
        "some random line\n{}\t{}\t{}\t{}\n{}\t{}\t{}\t{}\n",
        "1621350993",
        "f",
        "3330",
        "test/database-setup.sql",
        "1621350994",
        "f",
        "3334",
        "test/repomd.xml",
    );
    use std::fs;
    use std::fs::File;
    use std::io::Write;
    let mut f = File::create("test/fullfiletimelist-something").expect("Unable to create file");
    f.write_all(content.as_bytes())
        .expect("Unable to write data");

    if scan_local_directory(&mut cds, &[], "es", "test", false, "Test Category").is_err() {
        panic!();
    }
    for f in cds["test"].files.clone() {
        if f.name == "repomd.xml" && f.size == 3334 && f.timestamp == 1621350994 {
            repomd_found = true;
        }
    }

    for f in cds["test"].files.clone() {
        if f.name == "database-setup.sql" && f.size == 3330 && f.timestamp == 1621350993 {
            sql_found = true;
        }
    }

    assert!(repomd_found);
    assert!(sql_found);

    repomd_found = false;
    cds = HashMap::new();

    if scan_local_directory(&mut cds, &[], "es", "test", true, "Test Category").is_err() {
        panic!();
    }
    assert_eq!(cds.len(), 1);

    for f in cds["t"].files.clone() {
        if f.name == "repomd.xml" && f.size == 93 {
            repomd_found = true;
        }
    }

    if fs::remove_file("test/fullfiletimelist-something").is_err() {
        panic!();
    }

    assert!(repomd_found);
}

#[test]
fn scan_local_directory_fedora_linux_test() {
    use std::fs;
    use std::fs::File;
    use std::io::Write;

    // Create test directory structure for Fedora Linux category
    // Fedora Linux category expects fullfiletimelist one level up
    if fs::create_dir_all("test/fedora-linux/releases").is_err() {
        // Directory might already exist
    }

    let mut cds: HashMap<String, CategoryDirectory> = HashMap::new();

    // Create fullfiletimelist in parent directory with "linux/" prefixed paths
    let content = format!(
        "{}\t{}\t{}\t{}\n{}\t{}\t{}\t{}\n{}\t{}\t{}\t{}\n",
        "1621350993",
        "drwxr-xr-x",
        "4096",
        "linux/releases",
        "1621350994",
        "drwxr-xr-x",
        "4096",
        "linux/releases/42",
        "1621350995",
        "-rw-r--r--",
        "1234",
        "linux/releases/42/repomd.xml",
    );

    let mut f = File::create("test/fullfiletimelist-fedora").expect("Unable to create file");
    f.write_all(content.as_bytes())
        .expect("Unable to write data");

    // Test with "Fedora Linux" category - should find file in parent dir and strip prefix
    if scan_local_directory(
        &mut cds,
        &[],
        "releases",
        "test/fedora-linux",
        false,
        "Fedora Linux",
    )
    .is_err()
    {
        panic!();
    }

    // Verify the paths had "linux/" prefix stripped
    // The category directories should exist without "linux/" prefix
    let mut releases_dir_found = false;
    let mut releases_42_dir_found = false;
    let mut repomd_found = false;

    // Debug output
    for (key, _cd) in cds.iter() {
        println!("Category directory key: {}", key);
    }

    // Check that directories were created with stripped paths
    if cds.contains_key("releases") {
        releases_dir_found = true;
    }

    if cds.contains_key("releases/42") {
        releases_42_dir_found = true;
    }

    // Check in the specific category directory for the file
    if let Some(cd) = cds.get("releases/42") {
        for f in &cd.files {
            println!(
                "File in releases/42: {} (size: {}, timestamp: {})",
                f.name, f.size, f.timestamp
            );
            if f.name == "repomd.xml" && f.size == 1234 && f.timestamp == 1621350995 {
                repomd_found = true;
            }
        }
    }

    // Clean up
    if fs::remove_file("test/fullfiletimelist-fedora").is_err() {
        panic!();
    }
    if fs::remove_dir_all("test/fedora-linux").is_err() {
        // Ignore errors - directory might not exist
    }

    assert!(
        releases_dir_found,
        "releases directory should be found with prefix stripped"
    );
    assert!(
        releases_42_dir_found,
        "releases/42 directory should be found with prefix stripped"
    );
    assert!(
        repomd_found,
        "repomd.xml should be found with correct size and timestamp"
    );
}

#[test]
fn find_repositories_test() {
    let mut c = match get_db_connection() {
        Ok(c) => c,
        Err(e) => {
            println!("Database connection failed {}", e);
            panic!();
        }
    };

    // clean tables for test
    assert!(diesel::delete(db::schema::version::dsl::version)
        .execute(&mut c)
        .is_ok());

    let mut cds: HashMap<String, CategoryDirectory> = HashMap::new();
    if scan_with_rsync(&mut cds, &[], "", &[], &[], "test").is_err() {
        panic!();
    }
    assert_eq!(cds.len(), 1);
    cds.get_mut(&"test".to_string()).unwrap().ctime_changed = true;

    let category = db::functions::Category {
        id: 1,
        name: "Category".to_string(),
        topdir: "".to_string(),
        product_id: 5,
    };

    let repositories = vec![db::models::Repository {
        id: 1000,
        name: "RepositoryName".to_string(),
        prefix: Some("Prefix".to_string()),
        category_id: Some(1),
        version_id: Some(1),
        arch_id: Some(1),
        directory_id: Some(1),
        disabled: false,
    }];

    let rms = vec![settings::RepositoryMapping {
        regex: "[(^^^^".to_string(),
        prefix: "some".to_string(),
        version_prefix: None,
    }];

    let mut fds = db::functions::get_file_details(&mut c);
    let aliases = vec![settings::RepositoryAlias {
        from: "testing-modular-epel-debug-".to_string(),
        to: "testing-modular-debug-epel".to_string(),
    }];

    let mut find_parameter = FindRepositories {
        c: &mut c,
        cds: &mut cds,
        checksum_base: Some("http://localhost:17397/".to_string()),
        top: "".to_string(),
        cat: &category,
        repos: &repositories,
        rms: &rms,
        fds: &mut fds,
        skip_paths: &["skip".to_string()],
        test_paths: &["skip-test".to_string()],
        skip_repository_paths: &["skip".to_string()],
        do_not_display_paths: &["skip".to_string()],
        backend: "rsync".to_string(),
        aliases: &aliases,
    };
    if let Err(e) = find_repositories(&mut find_parameter) {
        println!("Creating repositories in database failed {}", e);
        panic!();
    }
    assert_eq!(
        fds.last().unwrap().sha256.as_ref().unwrap(),
        "55bd241dae474d89225650a0dd6446d21cbdccb607062e675543b91e074364a3"
    );
}
