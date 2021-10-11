// SPDX-License-Identifier: MIT

#[macro_use]
extern crate prettytable;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate derivative;
extern crate config;

#[macro_use]
extern crate serde_derive;

mod db;
mod debug;
mod settings;
mod xml;

use settings::Settings;

use regex::Regex;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::path::Path;
use std::process;
use std::process::Command;

use prettytable::format;

use diesel::pg::PgConnection;
use diesel::prelude::*;
use getopts::Options;

use chrono::prelude::*;

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

use walkdir::{DirEntry, WalkDir};

#[derive(Debug)]
struct FileInfo {
    is_directory: bool,
    is_readable: bool,
    size: i64,
    timestamp: i64,
    name: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct File {
    name: String,
    size: i64,
    timestamp: i64,
}

#[derive(Debug, Default, Clone)]
struct CategoryDirectory {
    files: Vec<File>,
    readable: bool,
    ctime: i64,
    directory_id: i32,
    // this is used if entries have been added to the database
    // either because they are new or because they have changed
    ctime_changed: bool,
}

struct UpdateDirectory {
    dir: db::models::Directory,
    ctime_changed: bool,
    readable_changed: bool,
    files_changed: bool,
}

/// Ages (cleans up) entries in the `file_detail` table.
///
/// For each file it is possible to have multiple entries in
/// the `file_detail` table. The main use case is to have
/// multiple entries (`<mm0:alternates>`) in the metalink
/// which gives mirrors time to pick up the most recent changes.
///
/// # Arguments
///
/// * `max_stale_days` - The number of days to keep additional entries
///   in the file_detail table
/// * `max_propagation_days` - The number of days the file has to
///   be propagated to all mirrors.
/// * `fds` - The list of all file_details which is used as the basis
///   for the clean up operation.
///
/// # Further Details
///
/// Usually `max_propagation_days` will be lesser than `max_stale_days`.
/// Both parameters can be controlled by the configuration file. If these
/// parameters are not set in the configuration file `max_stale_days`
/// defaults to 3 and `max_propagation_days` defaults to 2.
///
/// If the newest entry is at least `max_propagation_days` old all other
/// entries will be removed if older than `max_stale_days`.
///
/// If the newest entry is not older than `max_propagation_days` at least
/// two entries will be kept in the database.
///
/// The aged file_detail entries will directly be deleted from the
/// database. `fds` will not be updated to reflect the missing entries.
fn age_file_details(
    c: &PgConnection,
    fds: &mut Vec<db::models::FileDetail>,
    dirs: &[db::models::Directory],
    max_stale_days: i64,
    max_propagation_days: i64,
) -> Result<(), diesel::result::Error> {
    fds.sort_by(|a, b| {
        b.directory_id
            .cmp(&a.directory_id)
            .then(b.filename.cmp(&a.filename))
            .then(b.timestamp.cmp(&a.timestamp))
    });

    // At this point fds should be sorted by directory_id, filename and
    // timestamp descending. The newest entry should always come first.

    let mut old_id: i32 = -1;
    let mut old_ts: i64 = -1;
    let mut old_name = String::from("");
    let now = chrono::offset::Local::now().timestamp();
    let stale = now - (60 * 60 * 24 * max_stale_days);
    let propagation = now - (60 * 60 * 24 * max_propagation_days);
    let mut same_entries = 1;

    let mut delete_list: Vec<i32> = Vec::new();

    for fd in fds {
        let mut in_dirs = false;
        for d in dirs {
            if fd.directory_id == d.id {
                in_dirs = true;
                break;
            }
        }

        if !in_dirs {
            continue;
        }

        let ts = match fd.timestamp {
            Some(t) => t,
            _ => continue,
        };
        if fd.directory_id == old_id && fd.filename == old_name {
            same_entries += 1;
            if ts < stale && (same_entries > 2 || old_ts < propagation) {
                delete_list.push(fd.id);
            }
        } else {
            same_entries = 1;
            old_ts = ts;
        }
        old_id = fd.directory_id;
        old_name = fd.filename.clone();
    }

    if !delete_list.is_empty() {
        debug::STEPS.fetch_add(delete_list.len(), std::sync::atomic::Ordering::SeqCst);
    }

    for d in delete_list {
        let delete = diesel::delete(
            db::schema::file_detail::dsl::file_detail
                .filter(db::schema::file_detail::dsl::id.eq(d)),
        );
        let debug = diesel::debug_query::<diesel::pg::Pg, _>(&delete);
        debug::print_step(debug.to_string());
        delete.execute(c)?;
    }

    Ok(())
}

/// Returns a timestamp from rsync `date` and `time`
fn ctime_from_rsync(date: String, time: String) -> i64 {
    let dt: i64 =
        match Utc.datetime_from_str(format!("{} {}", date, time).as_str(), "%Y/%m/%d %H:%M:%S") {
            Ok(d) => d.timestamp(),
            Err(_) => -1,
        };

    dt
}

/// This should return the same `basename` from the shell
fn basename(path: String) -> String {
    let file_and_path: Vec<&str> = path.split('/').collect();
    let base = file_and_path.last().unwrap().to_string();

    base
}

/// This tries to figure out the version from a given path.
fn get_version_from_path(path: String) -> String {
    if path.contains("rawhide") {
        return String::from("development");
    }

    let pattern = Regex::new(r"/(([\.\d]+)([-_]\w+)?)/").unwrap();
    let version = match pattern.captures(&path) {
        Some(v) if v.len() > 1 => v.get(1).unwrap().as_str().to_string(),
        _ => String::from(""),
    };

    if !version.is_empty() {
        return version;
    }

    String::from("")
}

/// Guess version and architecutre from a given path.
///
/// This returns the version name, the version ID and
/// the architecutre ID as found in the database. Both IDs
/// are necessary to add a new entry to the table `repository`.
///
/// If the found version does not exist in the database it is added
/// to the database and to the parameter `versions`. This way the
/// database has not to be contacted to update `versions`.
fn guess_ver_arch_from_path(
    c: &PgConnection,
    path: String,
    arches: &[db::models::Arch],
    versions: &mut Vec<db::models::Version>,
    product_id: i32,
    test_paths: &[String],
    do_not_display_paths: &[String],
) -> Result<(String, i32, i32), Box<dyn Error>> {
    let mut arch_id: i32 = -1;
    let mut version_id: i32 = -1;
    let mut version_name = String::new();

    for a in arches {
        let pattern = Regex::new(format!(r".*(^|/){}(/|$).*", &a.name).as_str()).unwrap();
        if pattern.is_match(&path) {
            arch_id = a.id;
            break;
        }
    }

    if arch_id == -1 && (path.contains("SRPMS") || path.contains("/src")) {
        for a in arches {
            if a.name == "source" {
                arch_id = a.id;
                break;
            }
        }
    }

    if arch_id == -1 {
        println!("Not able to figure out architecture from {}", path);
        return Err("Not able to figure out architecture".into());
    }

    for v in versions.clone() {
        let pattern = Regex::new(format!(r".*(^|/){}(/|$).*", &v.name).as_str()).unwrap();
        if pattern.is_match(&path) && product_id == v.product_id {
            version_id = v.id;
            if v.name == *"development" {
                // This, just like the SRPMS <-> source connection above is a bit
                // unfortunate and hard coded for now. Can easily be pulled into
                // the configuration file if necessary.
                // In the database the version is called 'development' but the
                // repository prefix uses 'rawhide' as the version string.
                version_name = "rawhide".to_string();
            } else {
                version_name = v.name;
            }
        }
    }

    if version_id == -1 {
        let version = get_version_from_path(path.clone());
        if !version.is_empty() {
            // Version does not exist yet in the database. Let's create it
            let mut is_test = false;
            let mut display = true;
            for tp in test_paths {
                if path.contains(tp) {
                    is_test = true;
                    break;
                }
            }
            for tp in do_not_display_paths {
                if path.contains(tp) {
                    display = false;
                    break;
                }
            }
            let insert = diesel::insert_into(db::schema::version::dsl::version).values((
                db::schema::version::dsl::product_id.eq(product_id),
                db::schema::version::dsl::name.eq(version),
                db::schema::version::dsl::sortorder.eq(0),
                db::schema::version::dsl::is_test.eq(is_test),
                db::schema::version::dsl::display.eq(display),
                db::schema::version::dsl::ordered_mirrorlist.eq(true),
            ));
            debug::STEPS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

            let debug = diesel::debug_query::<diesel::pg::Pg, _>(&insert);
            debug::print_step(debug.to_string());
            let result = insert.get_results::<db::models::InsertVersion>(c)?;

            versions.push(db::models::Version {
                id: result[0].id,
                name: result[0].name.clone(),
                product_id: result[0].product_id,
                is_test: result[0].is_test,
            });
            version_id = result[0].id;
            version_name = result[0].name.clone();
        }
    }

    Ok((version_name, version_id, arch_id))
}

/// Return the repository prefix based on the `path`, `version` and `rms`.
///
/// * `rms` - this is the repositroy mapping as found in the configuration file.
///   It consists of a regex and a prefix. If a patch matches the regex the given
///   prefix is used to create the repository prefix.
///
/// The returned repository_prefix consists of the prefix from the configuration file
/// to which the version is added and if appropriate `-source` or `-debug`.
fn repo_prefix(
    path: String,
    version: String,
    rms: &[settings::RepositoryMapping],
    aliases: &[settings::RepositoryAlias],
) -> String {
    let mut is_source_or_debug = String::new();

    if path.contains("/source") || path.contains("/SRPMS") || path.contains("/src") {
        is_source_or_debug = String::from("-source");
    } else if path.contains("/debug") {
        is_source_or_debug = String::from("-debug");
    }

    for rm in rms {
        let pattern = match Regex::new(rm.regex.as_str()) {
            Ok(p) => p,
            Err(_) => {
                println!("Cannot handle regex {}. Skipping", rm.regex);
                continue;
            }
        };

        if pattern.is_match(&path) {
            if version == *"rawhide" {
                return format!("{}-{}{}", rm.prefix, version, is_source_or_debug);
            } else {
                let mut prefix = format!(
                    "{}{}-",
                    match rm.prefix.contains('$') {
                        // A '$' means that this contains a replacement group. Probably.
                        true => pattern.replace(&path, rm.prefix.clone()).to_string(),
                        _ => rm.prefix.clone(),
                    },
                    is_source_or_debug
                );

                for a in aliases {
                    if prefix == a.from {
                        prefix = a.to.clone();
                    }
                }
                return format!("{}{}", prefix, version);
            }
        }
    }

    String::from("")
}

/// Check if there already is a repository for this prefix and architecture
///
/// This functions goes through `repos` to see if there is already a
/// repository with `prefix` and `arch_id`.
fn check_for_repo(repos: &[db::models::Repository], prefix: String, arch_id: i32) -> bool {
    for r in repos {
        let db_prefix = match &r.prefix {
            Some(p) => p,
            _ => continue,
        };
        let db_arch_id = match &r.arch_id {
            Some(a) => a,
            _ => continue,
        };
        if *db_prefix == prefix && *db_arch_id == arch_id {
            return true;
        }
    }

    false
}

/// Remove non-existing directories from the database
///
/// If a directory has been deleted on the file system it will still exist in the database. This
/// function goes through the list of database directories and if it does not exist on the file
/// system it will be removed from `category_directory`, `host_category_dir`, `directory`,
/// `repository' and `file_detail`.
fn cleanup_database(
    c: &PgConnection,
    cds: &HashMap<String, CategoryDirectory>,
    dirs: &[db::models::Directory],
    topdir: String,
) -> Result<usize, diesel::result::Error> {
    for d in dirs {
        let mut dir_gone_from_fs = true;

        for k in cds.keys() {
            let mut with_topdir = format!("{}{}", topdir, k);
            if k.is_empty() {
                with_topdir.pop();
            }

            if d.name == with_topdir {
                dir_gone_from_fs = false;
                break;
            }
        }

        if dir_gone_from_fs {
            debug::STEPS.fetch_add(5, std::sync::atomic::Ordering::SeqCst);
            // Delete from CategoryDirectory (Is it possible to delete multiple entries at once???)
            // Something like 'DELETE FROM category_directory where directory_id = 10 or directory_id = 20'.
            let delete_cd = diesel::delete(
                db::schema::category_directory::dsl::category_directory
                    .filter(db::schema::category_directory::dsl::directory_id.eq(d.id)),
            );
            let debug_cd = diesel::debug_query::<diesel::pg::Pg, _>(&delete_cd);
            debug::print_step(debug_cd.to_string());
            delete_cd.execute(c)?;

            // Delete from HostCategoryDir
            let delete_host_category_dir = diesel::delete(
                db::schema::host_category_dir::dsl::host_category_dir
                    .filter(db::schema::host_category_dir::dsl::directory_id.eq(d.id)),
            );
            let debug_host_category_dir =
                diesel::debug_query::<diesel::pg::Pg, _>(&delete_host_category_dir);
            debug::print_step(debug_host_category_dir.to_string());
            delete_host_category_dir.execute(c)?;

            // Delete from Repository
            let delete_repository = diesel::delete(
                db::schema::repository::dsl::repository
                    .filter(db::schema::repository::dsl::directory_id.eq(d.id)),
            );
            let debug_repository = diesel::debug_query::<diesel::pg::Pg, _>(&delete_repository);
            debug::print_step(debug_repository.to_string());
            delete_repository.execute(c)?;

            // And remove if from FileDetail
            let delete_fd = diesel::delete(
                db::schema::file_detail::dsl::file_detail
                    .filter(db::schema::file_detail::dsl::directory_id.eq(d.id)),
            );
            let debug_fd = diesel::debug_query::<diesel::pg::Pg, _>(&delete_fd);
            debug::print_step(debug_fd.to_string());
            delete_fd.execute(c)?;

            // Delete from Directory
            let delete_dir = diesel::delete(
                db::schema::directory::dsl::directory
                    .filter(db::schema::directory::dsl::id.eq(d.id)),
            );
            let debug_dir = diesel::debug_query::<diesel::pg::Pg, _>(&delete_dir);
            debug::print_step(debug_dir.to_string());
            delete_dir.execute(c)?;
        }
    }
    Ok(0)
}

#[derive(Debug)]
struct DetailsResult {
    md5_sum: String,
    sha1_sum: String,
    sha256_sum: String,
    sha512_sum: String,
    length: i64,
    timestamp: i64,
    target: String,
}

fn get_file_content(
    base: &Option<String>,
    topdir: &str,
    dir: &str,
    target: &str,
    backend: &str,
) -> Result<(String, i64), Box<dyn Error>> {
    use std::fs;

    let backend_config_name = match backend {
        "directory" => "local_prefix",
        "rsync" => "checksum_base",
        _ => return Err(format!("Do not know how to handle backend '{}'", backend).into()),
    };

    let full_target = format!(
        "{}{}{}/{}",
        match base {
            Some(c) => c,
            _ =>
                return Err(format!(
                    "For backend '{}' '{}' needs to be set",
                    backend, backend_config_name
                )
                .into()),
        },
        topdir,
        dir,
        target,
    );

    if backend == "directory" {
        return Ok((
            fs::read_to_string(&full_target)?,
            fs::metadata(full_target)?.len() as i64,
        ));
    }

    let resp = match reqwest::blocking::get(&full_target) {
        Ok(r) => r,
        Err(e) => return Err(format!("Error '{}' retrieving '{}'", e, full_target).into()),
    };
    let content_length: i64 = match resp.headers().get("content-length") {
        Some(r) => r.to_str()?.parse::<i64>()?,
        _ => 0,
    };

    Ok((resp.text()?, content_length))
}

fn get_details(
    checksum_base: &Option<String>,
    topdir: &str,
    dir: &str,
    target: &str,
    backend: &str,
) -> Result<Vec<DetailsResult>, Box<dyn Error>> {
    let (body, content_length) = get_file_content(checksum_base, topdir, dir, target, backend)?;

    use md5::{Digest, Md5};
    use sha1::Sha1;
    use sha2::{Sha256, Sha512};

    let mut md5 = Md5::new();
    md5.update(&body);
    let mut sha1 = Sha1::new();
    sha1.update(&body);
    let mut sha256 = Sha256::new();
    sha256.update(&body);
    let mut sha512 = Sha512::new();
    sha512.update(&body);

    return Ok(vec![DetailsResult {
        md5_sum: format!("{:x}", md5.finalize()),
        sha1_sum: format!("{:x}", sha1.finalize()),
        sha256_sum: format!("{:x}", sha256.finalize()),
        sha512_sum: format!("{:x}", sha512.finalize()),
        length: content_length,
        timestamp: xml::get_timestamp(body),
        target: target.to_string(),
    }]);
}

fn get_details_via_checksum_file(
    checksum_base: &Option<String>,
    topdir: &str,
    dir: &str,
    target: &str,
    backend: &str,
    files: &Option<Vec<File>>,
) -> Result<Vec<DetailsResult>, Box<dyn Error>> {
    let (body, _) = get_file_content(checksum_base, topdir, dir, target, backend)?;

    let mut drs: Vec<DetailsResult> = Vec::new();

    if files.is_none() {
        return Ok(Vec::new());
    }

    'outer: for line in body.lines() {
        let elements = line.split(' ').collect::<Vec<_>>();
        if elements.len() == 4 {
            // We are looking for lines like this
            // SHA256 (Fedora-Cloud-Base-34_Beta-1.3.x86_64.qcow2) = e2f87e760162a596a0adeb14d4d563fcad25b5954d3064d98eefda24691574b5
            // Currently only SHA256 is used in the files so let's look for that and SHA512
            if elements[0] != "SHA256" && elements[0] != "SHA512" {
                continue 'outer;
            }

            // Check if the file exists
            let mut file = File::default();
            let file_name = elements[1].trim_start_matches('(').trim_end_matches(')');
            for f in files.as_ref().unwrap() {
                if f.name == file_name {
                    file = f.clone();
                    break;
                }
            }
            if file.name.is_empty() {
                continue 'outer;
            }

            let dr = DetailsResult {
                md5_sum: String::new(),
                sha1_sum: String::new(),
                sha256_sum: match elements[0] {
                    "SHA256" => elements[3].to_string(),
                    _ => String::new(),
                },
                sha512_sum: match elements[0] {
                    "SHA512" => elements[3].to_string(),
                    _ => String::new(),
                },
                length: file.size,
                timestamp: file.timestamp,
                target: file.name,
            };
            drs.push(dr);
        }
    }

    Ok(drs)
}

/// Parameter for the `fill_ifds()` funcion
struct FillIfds<'a> {
    /// InsertFileDetail return vector
    ifds: &'a mut Vec<db::models::InsertFileDetail>,
    /// The name of the checksum target file
    target: &'a str,
    /// Which backend this is using
    backend: &'a str,
    /// The base for creating checksums. For rsync a http URL
    /// For file a directory path
    checksum_base: &'a Option<String>,
    /// topdir as defined by the category
    topdir: &'a str,
    /// The current directory this is about
    dir: &'a str,
    /// The directory for newly created entries
    d_id: i32,
    /// The currently in the database existing entries
    fds: &'a [db::models::FileDetail],
    /// The list of files in this directory. Only used
    /// for '-CHECKSUM' files.
    files: &'a Option<Vec<File>>,
}

fn fill_ifds(p: &mut FillIfds) -> Result<(), Box<dyn Error>> {
    let drs_result = match p.backend {
        "rsync" | "directory" if p.target.ends_with("-CHECKSUM") => get_details_via_checksum_file(
            p.checksum_base,
            p.topdir,
            p.dir,
            p.target,
            p.backend,
            p.files,
        ),
        "rsync" | "directory" => get_details(p.checksum_base, p.topdir, p.dir, p.target, p.backend),
        _ => return Err(format!("Unsupported scan backend '{}'", p.backend).into()),
    };

    let drs = match drs_result {
        Ok(d) => d,
        Err(e) => {
            println!(
                "Getting file details for {} via {} failed: {}. Skipping.",
                p.target, p.backend, e
            );
            return Ok(());
        }
    };

    for dr in drs {
        let mut found_in_db = false;

        // find repomd.xml in file_details
        for fd in p.fds {
            let timestamp_db = match fd.timestamp {
                Some(t) => t,
                _ => 0,
            };

            let size_db = match fd.size {
                Some(s) => s,
                _ => 0,
            };

            let sha1_db = match &fd.sha1 {
                Some(s) => String::from(s),
                _ => String::from(""),
            };

            let md5_db = match &fd.md5 {
                Some(s) => String::from(s),
                _ => String::from(""),
            };

            let sha256_db = match &fd.sha256 {
                Some(s) => String::from(s),
                _ => String::from(""),
            };

            let sha512_db = match &fd.sha512 {
                Some(s) => String::from(s),
                _ => String::from(""),
            };

            if fd.directory_id == p.d_id
                && fd.filename == dr.target
                && size_db == dr.length
                && timestamp_db == dr.timestamp
                && sha1_db == dr.sha1_sum
                && md5_db == dr.md5_sum
                && sha256_db == dr.sha256_sum
                && sha512_db == dr.sha512_sum
            {
                found_in_db = true;
            }
        }

        if !found_in_db {
            p.ifds.push(db::models::InsertFileDetail {
                directory_id: p.d_id,
                filename: dr.target.to_string(),
                timestamp: Some(dr.timestamp),
                size: Some(dr.length),
                sha1: Some(dr.sha1_sum),
                md5: Some(dr.md5_sum),
                sha256: Some(dr.sha256_sum),
                sha512: Some(dr.sha512_sum),
            });
        }
    }

    Ok(())
}

/// Parameter for the `find_repositories()` function.
struct FindRepositories<'a> {
    /// The connection to the database
    c: &'a PgConnection,
    /// The hashmap of the file system scan
    cds: &'a mut HashMap<String, CategoryDirectory>,
    /// For rsync based scans a HTTP(S) URL where files can
    /// be downloaded from for checksum creation.
    /// For file based scans the path on the local file system
    /// which contains all the files.
    checksum_base: Option<String>,
    top: String,
    /// The category all these files belong to
    cat: &'a db::functions::Category,
    /// List of existing repositories which will be used
    /// to decide if a new repository needs to be created.
    repos: &'a [db::models::Repository],
    /// Repository mappings from the configuration file
    /// which will be used to create repository prefixes.
    rms: &'a [settings::RepositoryMapping],
    /// The content from the table `file_detail` which will be
    /// amended if a new repomd.xml file has been found.
    fds: &'a mut Vec<db::models::FileDetail>,
    /// List of directory prefixes which should be ignored
    /// when trying to find repositories.
    skip_paths: &'a [String],
    /// If a path contains one of the following strings the
    /// repository will be marked as a test release.
    test_paths: &'a [String],
    /// If one of the following strings is part of the repository
    /// path the repository creation will be skipped.
    skip_repository_paths: &'a [String],
    /// If one of the following strings is part of the path
    /// a newly created version will be set to display = false
    do_not_display_paths: &'a [String],
    /// Which backend is used to scan the primary mirror
    backend: String,
    /// List of Repository aliases for some repositories not
    /// following the default naming scheme.
    aliases: &'a [settings::RepositoryAlias],
}

/// Find repositories in the list of scanned directories.
///
/// Based on the input structure `FindRepositories` this
/// function will create new repository objects in the database.
fn find_repositories(p: &mut FindRepositories) -> Result<usize, Box<dyn Error>> {
    if p.backend != "rsync" && p.backend != "directory" {
        return Err(format!("Cannot handle backend type {}", p.backend).into());
    }

    let mut ifds: Vec<db::models::InsertFileDetail> = Vec::new();

    let arches = db::functions::get_arches(p.c)?;
    let mut versions = db::functions::get_versions(p.c)?;
    let fds = p.fds.clone();

    let list: Vec<String> = p.cds.keys().cloned().collect();
    'outer: for k in list {
        // No need to look at unchanged entries
        if !p.cds.get(&k).unwrap().ctime_changed {
            continue;
        }
        // Let's go over the files in this directory to see if there
        // is a file ending with '-CHECKSUM' which might contain
        // checksums.
        for f in &p.cds.get(&k).unwrap().files {
            // Currently only files ending in '-CHECKSUM' exist
            if f.name.ends_with("-CHECKSUM") {
                // We found such a file. Now let's parse it and add it
                // to 'ifds' to be later added to the database.
                println!("Found CHECKSUM {}", f.name);
                fill_ifds(&mut FillIfds {
                    ifds: &mut ifds,
                    target: &f.name,
                    backend: &p.backend,
                    checksum_base: &p.checksum_base,
                    topdir: &p.top,
                    dir: &k,
                    d_id: p.cds[&k].directory_id,
                    fds: &fds,
                    files: &Some(p.cds.get(&k).unwrap().files.clone()),
                })?;
            }
        }
        if basename(k.to_string()) == *"repodata" {
            for s in p.skip_repository_paths {
                if k.contains(s) {
                    continue 'outer;
                }
            }
            let parent = String::from(Path::new(&k).parent().unwrap().to_str().unwrap());
            // 'parent' should always be a key of cds
            let cd = p.cds.get(&parent).unwrap();
            let with_topdir = match parent.is_empty() {
                true => p.top.clone(),
                false => format!("{}{}", p.top, parent),
            };

            for s in p.skip_paths {
                if with_topdir.starts_with(s) {
                    // Never create a repository for this path
                    continue 'outer;
                }
            }

            let (version_name, version_id, arch_id) = guess_ver_arch_from_path(
                p.c,
                with_topdir.clone(),
                &arches,
                &mut versions,
                p.cat.product_id,
                p.test_paths,
                p.do_not_display_paths,
            )?;
            if version_id == -1 {
                println!("Not able to guess version for {}", with_topdir);
                println!("Not creating repository in database");
                continue;
            }
            let prefix = repo_prefix(with_topdir.clone(), version_name, p.rms, p.aliases);
            if prefix.is_empty() {
                println!("Not able to determine prefix for {}", with_topdir.clone());
            }
            if !check_for_repo(p.repos, prefix.clone(), arch_id) {
                if let Err(e) = db::functions::create_repository(
                    p.c,
                    cd.directory_id,
                    with_topdir,
                    p.cat.id,
                    version_id,
                    arch_id,
                    prefix.clone(),
                ) {
                    println!(
                        "Repository creation failed for {}: {}. Skipping.",
                        prefix, e
                    );
                    continue;
                }
            }

            fill_ifds(&mut FillIfds {
                ifds: &mut ifds,
                target: "repomd.xml",
                backend: &p.backend,
                checksum_base: &p.checksum_base,
                topdir: &p.top,
                dir: &k,
                d_id: p.cds[&k].directory_id,
                fds: &fds,
                files: &None,
            })?;
        }
    }

    if !ifds.is_empty() {
        let insert = diesel::insert_into(db::schema::file_detail::dsl::file_detail).values(&ifds);

        debug::STEPS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let debug = diesel::debug_query::<diesel::pg::Pg, _>(&insert);
        debug::print_step(debug.to_string());

        let result = insert.get_results::<db::models::FileDetail>(p.c)?;
        for r in result {
            p.fds.push(r.clone());
        }
    }

    Ok(0)
}

fn is_excluded(path: String, excludes: &[String]) -> bool {
    for ex in excludes {
        let pattern = match Regex::new(ex.as_str()) {
            Ok(p) => p,
            Err(_) => {
                println!("Cannot handle exclude regex {}. Skipping", ex);
                continue;
            }
        };

        if pattern.is_match(&path) {
            println!("{} is excluded because of {}", path, ex);
            return true;
        }
    }

    false
}

fn add_entry_to_category_directories(
    fi: FileInfo,
    cds: &mut HashMap<String, CategoryDirectory>,
    excludes: &[String],
    topdir: &str,
) {
    let name = match fi.name {
        Some(n) => n,
        _ => return,
    };

    let base = basename(String::from(&name));
    let dir = match Path::new(&name).parent() {
        Some(parent) => match parent.to_str() {
            Some(p) => match fi.is_directory {
                true if base == "." => String::from(""),
                true => name,
                _ => String::from(p),
            },
            _ => {
                println!("Failed getting parent path for {}", &name);
                return;
            }
        },
        _ => String::from(""),
    };

    let with_topdir = match dir.is_empty() {
        true => String::from(topdir),
        false => format!("{}{}", topdir, dir),
    };

    if is_excluded(with_topdir, excludes) {
        return;
    }

    let cd: &mut CategoryDirectory = match cds.get_mut(&dir) {
        Some(v) => &mut *v,
        None => {
            cds.insert(String::from(&dir), CategoryDirectory::default());
            &mut *cds.get_mut(&dir).unwrap()
        }
    };

    if fi.is_directory {
        cd.ctime = fi.timestamp;
        cd.readable = fi.is_readable;
    } else {
        let file = File {
            name: base,
            size: fi.size,
            timestamp: fi.timestamp,
        };
        cd.files.push(file);
    }
}

fn list_categories(cl: &[db::functions::Category]) {
    let mut table = prettytable::Table::new();
    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);

    table.set_titles(row!["Category Name", "Category top directory"]);

    for c in cl {
        table.add_row(row![c.name, c.topdir]);
    }

    table.printstd();
}

// This function returns a JSON string of the newest
// files in the given directory.
// If there are moren than max (10) of either .rpm
// or .html files only the newest 10 files are
// returned.
// This is the list of files the crawler will search for.
fn short_filelist(cd: &CategoryDirectory) -> String {
    let mut files = cd.files.clone();
    let mut html = 0;
    let mut rpm = 0;
    let limit: usize;
    let max = 10;
    files.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    for f in &files {
        if f.name.ends_with(".html") {
            html += 1;
        }
        if f.name.ends_with(".rpm") {
            rpm += 1;
        }
    }
    if rpm > max || html > max {
        limit = max;
    } else {
        limit = files.len();
    }
    match serde_json::to_string(&files[0..limit]) {
        Ok(j) => j,
        _ => String::new(),
    }
}

fn update_category_directory(
    c: &PgConnection,
    uc: &[db::models::Directory],
    cat_id: i32,
) -> Result<usize, diesel::result::Error> {
    use db::schema::category_directory;
    use db::schema::category_directory::dsl::*;
    #[derive(Insertable)]
    #[table_name = "category_directory"]
    struct InsertCategoryDirectory<'a> {
        category_id: &'a i32,
        directory_id: &'a i32,
    }

    let mut new_cds: Vec<InsertCategoryDirectory> = Vec::new();
    for i in uc {
        new_cds.push(InsertCategoryDirectory {
            category_id: &cat_id,
            directory_id: &i.id,
        });
    }

    let insert = diesel::insert_into(category_directory).values(&new_cds);

    debug::STEPS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let debug = diesel::debug_query::<diesel::pg::Pg, _>(&insert);
    debug::print_step(debug.to_string());

    insert.execute(c)
}

fn add_directories(
    c: &PgConnection,
    ad: &HashMap<String, CategoryDirectory>,
    cat_id: i32,
) -> Result<Vec<db::models::Directory>, diesel::result::Error> {
    if ad.is_empty() {
        return Ok(Vec::new());
    }

    use db::schema::directory;
    use db::schema::directory::dsl::*;
    #[derive(Insertable, Derivative)]
    #[derivative(Debug)]
    #[table_name = "directory"]
    struct InsertDirectory<'a> {
        readable: &'a bool,
        ctime: &'a i64,
        #[derivative(Debug = "ignore")]
        files: Vec<u8>,
        name: &'a String,
    }

    let mut new_directories: Vec<InsertDirectory> = Vec::new();
    for k in ad.keys() {
        new_directories.push(InsertDirectory {
            readable: &ad[k].readable,
            ctime: &ad[k].ctime,
            name: k,
            files: short_filelist(&ad[k]).as_bytes().to_vec(),
        });
    }

    let insert = diesel::insert_into(directory).values(&new_directories);

    debug::STEPS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    debug::print_step(format!("INSERT INTO directory {:?}", new_directories));

    let result = insert.get_results::<db::models::Directory>(c)?;

    update_category_directory(c, &result, cat_id)?;

    Ok(result)
}

fn sync_category_directories(
    c: &PgConnection,
    topdir: String,
    cat_id: i32,
    dirs: &mut Vec<db::models::Directory>,
    cds: &mut HashMap<String, CategoryDirectory>,
) -> Result<(), Box<dyn Error>> {
    let mut update_directories: Vec<UpdateDirectory> = Vec::new();
    let mut ad: HashMap<String, CategoryDirectory> = HashMap::new();
    for k in cds.clone().keys() {
        let mut not_found = true;
        let cd = cds.get_mut(k).unwrap();
        let mut with_topdir = format!("{}{}", topdir, k);
        if k.is_empty() {
            with_topdir.pop();
        }
        // Avoiding clone() would be nice. Not sure how.
        for d in dirs.clone() {
            if d.name == with_topdir {
                not_found = false;
                let ctime_changed = cd.ctime != d.ctime;
                let readable_changed = cd.readable != d.readable;
                if ctime_changed || readable_changed {
                    let mut entry = UpdateDirectory {
                        dir: d.clone(),
                        ctime_changed: cd.ctime != d.ctime,
                        readable_changed: cd.readable != d.readable,
                        files_changed: false,
                    };
                    if ctime_changed {
                        entry.dir.ctime = cd.ctime;
                        cd.ctime_changed = true;
                    }
                    if readable_changed {
                        entry.dir.readable = cd.readable;
                    }
                    if readable_changed || ctime_changed {
                        let json = short_filelist(cd);
                        if d.files != json.as_bytes() {
                            entry.dir.files = json.as_bytes().to_vec();
                            entry.files_changed = true;
                        }
                    }
                    entry.dir.id = d.id;
                    update_directories.push(entry);
                }
                cd.directory_id = d.id;
                break;
            }
        }
        if not_found {
            ad.insert(with_topdir, cd.clone());
        }
    }

    debug::STEPS.fetch_add(
        update_directories.len(),
        std::sync::atomic::Ordering::SeqCst,
    );

    let new_dirs = add_directories(c, &ad, cat_id)?;
    for i in &new_dirs {
        dirs.push(i.clone());

        cds.get_mut(i.name.trim_start_matches(&topdir))
            .unwrap()
            .directory_id = i.id;
        // Track that this is a new entry
        cds.get_mut(i.name.trim_start_matches(&topdir))
            .unwrap()
            .ctime_changed = true;
    }

    for u in update_directories {
        use db::schema::directory;
        use db::schema::directory::dsl::*;
        #[derive(AsChangeset, Default, Derivative)]
        #[derivative(Debug)]
        #[table_name = "directory"]
        struct UpdateDirectory<'a> {
            readable: Option<&'a bool>,
            ctime: Option<&'a i64>,
            #[derivative(Debug = "ignore")]
            files: Option<&'a Vec<u8>>,
        }

        let mut ud = UpdateDirectory::default();

        if u.ctime_changed {
            ud.ctime = Some(&u.dir.ctime);
        }
        if u.readable_changed {
            ud.readable = Some(&u.dir.readable);
        }
        if u.files_changed {
            ud.files = Some(&u.dir.files);
        }

        let target = directory.filter(db::schema::directory::dsl::id.eq(u.dir.id));

        let update = diesel::update(target).set(&ud);

        debug::print_step(format!("UPDATE directory {:?} where ID = {}", ud, u.dir.id));

        update.execute(c)?;
    }

    Ok(())
}

fn handle_unreadable(cds: &mut HashMap<String, CategoryDirectory>) {
    let mut cds_keys: Vec<String> = cds.keys().cloned().collect::<Vec<String>>();
    cds_keys.sort();

    let cds_copy = cds.clone();

    for d in cds_keys {
        let mut cd = match cds.get_mut(&d) {
            Some(c) => c,
            _ => continue,
        };

        let parent: String = match Path::new(&d).parent() {
            Some(p) => match p.to_str() {
                Some(pp) => pp.to_string(),
                _ => continue,
            },
            _ => continue,
        };

        let parent_entry = match cds_copy.get(&parent) {
            Some(p) => p,
            _ => continue,
        };
        if !parent_entry.readable && cd.readable {
            cd.readable = false;
        }
    }
}

fn is_not_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| entry.depth() == 0 || !s.starts_with('.'))
        .unwrap_or(false)
}

fn scan_local_directory(
    cds: &mut HashMap<String, CategoryDirectory>,
    excludes: &[String],
    topdir: &str,
    url: &str,
    skip_fftl: bool,
) -> Result<(), Box<dyn Error>> {
    use glob::glob;
    use std::os::unix::fs::PermissionsExt;

    let fullfiletimelist = match glob(format!("{}/fullfiletimelist-*", url).as_str())?.next() {
        Some(Ok(fftm)) => fftm.into_os_string().into_string().unwrap(),
        _ => "".to_string(),
    };

    if !fullfiletimelist.is_empty() && !skip_fftl {
        debug::print_step(format!(
            "Local directory ({}) scan using {}",
            url, fullfiletimelist
        ));
        let file = std::fs::File::open(fullfiletimelist)?;
        let data = unsafe { memmap::MmapOptions::new().map(&file)? };
        for line in data.split(|elem| elem == &b'\n') {
            let v: Vec<&str> = std::str::from_utf8(line)?.split('\t').collect();
            if v.len() < 4 {
                continue;
            }
            let info = FileInfo {
                is_directory: v[1].starts_with('d'),
                is_readable: !v[1].contains('-'),
                size: v[2].parse()?,
                timestamp: v[0].parse()?,
                name: Some(v[3].to_string()),
            };
            add_entry_to_category_directories(info, cds, excludes, topdir);
        }

        return Ok(());
    }

    debug::print_step(format!("Local directory ({}) scan", url));
    WalkDir::new(url)
        .into_iter()
        .filter_entry(|e| is_not_hidden(e))
        .filter_map(|v| v.ok())
        .map(|info| FileInfo {
            is_directory: info.file_type().is_dir(),
            is_readable: (info.metadata().unwrap().permissions().mode() & 0o4) != 0,
            size: info.metadata().unwrap().len() as i64,
            timestamp: info
                .metadata()
                .unwrap()
                .modified()
                .unwrap()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            name: info
                .path()
                .display()
                .to_string()
                .split(topdir)
                .map(str::to_string)
                .collect::<Vec<String>>()
                .get(1)
                .cloned(),
        })
        .for_each(|x| {
            add_entry_to_category_directories(x, cds, excludes, topdir);
        });

    Ok(())
}

fn scan_with_rsync(
    cds: &mut HashMap<String, CategoryDirectory>,
    excludes: &[String],
    topdir: &str,
    category_rsync_options: &[String],
    rsync_options: &[String],
    url: &str,
) -> Result<(), Box<dyn Error>> {
    debug::print_step(format!(
        "Running rsync -r --no-human-readable {:?} {:?} {}",
        &rsync_options, &category_rsync_options, url
    ));

    let output = Command::new("rsync")
        // We always need '-r' and '--no-human-readable'
        .arg("-r")
        .arg("--no-human-readable")
        .args(rsync_options)
        .args(category_rsync_options)
        .arg(url)
        .output();

    let pattern = Regex::new(r"([drwSsx-]{10})\s*(.*) (.*) (.*) (.*)")?;

    String::from_utf8(output?.stdout)?
        .lines()
        .filter_map(|line| pattern.captures(line))
        .map(|info| FileInfo {
            is_directory: info[1].starts_with('d'),
            is_readable: Regex::new(r"^d......r.x").unwrap().is_match(&info[1]),
            size: info[2].parse().unwrap(),
            timestamp: ctime_from_rsync(info[3].to_string(), info[4].to_string()),
            name: info.get(5).map(|n| n.as_str().trim().to_string()),
        })
        .for_each(|x| {
            add_entry_to_category_directories(x, cds, excludes, topdir);
        });

    Ok(())
}

struct Parameters {
    list_categories: bool,
    category_specified: bool,
    category_name: String,
    delete_directories: bool,
    config_file: String,
    skip_fftl: bool,
}

fn setup_params() -> Parameters {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();
    let mut opts = Options::new();

    let mut params = Parameters {
        list_categories: false,
        category_specified: false,
        category_name: "".to_string(),
        delete_directories: false,
        config_file: String::from("/etc/mirrormanager/scan-primary-mirror.toml"),
        skip_fftl: false,
    };

    opts.optmulti(
        "c",
        "config",
        &format!("configuration file ({})", params.config_file),
        "CONFIG",
    );
    opts.optflagmulti("d", "debug", "enable debug");
    opts.optflagmulti("", "list-categories", "list available catagories");
    opts.optflagmulti(
        "",
        "delete-directories",
        "delete directories from the database that no longer exist",
    );
    opts.optflagmulti(
        "",
        "skip-fullfiletimelist",
        "do not look for a fullfiletimelist-*; actually scan the filesystem",
    );

    opts.optmulti("", "category", "only scan category CATEGORY", "CATEGORY");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        _ => {
            print!(
                "{}",
                opts.usage(format!("Usage: {} [options]", program).as_str())
            );
            process::exit(0);
        }
    };

    if matches.opt_present("debug") {
        debug::DEBUG.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    if matches.opt_present("list-categories") {
        params.list_categories = true;
    }

    if matches.opt_present("skip-fullfiletimelist") {
        params.skip_fftl = true;
    }

    if matches.opt_present("category") {
        params.category_specified = true;
        params.category_name =
            matches.opt_strs("category")[matches.opt_count("category") - 1].to_string();
    }

    if matches.opt_present("delete-directories") {
        params.delete_directories = true;
    }

    if matches.opt_present("config") {
        params.config_file =
            matches.opt_strs("config")[matches.opt_count("config") - 1].to_string();
    }

    params
}

fn main() {
    let mut category = db::functions::Category {
        id: -1,
        name: "".to_string(),
        topdir: "".to_string(),
        product_id: -1,
    };

    let params = setup_params();

    let settings = match Settings::new(params.config_file) {
        Ok(s) => s,
        Err(e) => {
            println!("Configuration file parsing failed: {}", e);
            process::exit(1);
        }
    };

    let connection = match PgConnection::establish(&settings.database.url) {
        Ok(c) => c,
        Err(e) => {
            println!("Connection to the database failed: {}", e);
            process::exit(1);
        }
    };

    let cl = db::functions::get_categories(&connection);

    if params.list_categories {
        list_categories(&cl);
        process::exit(0);
    }

    if !params.category_specified {
        println!("Please specify a category using '--category'\n");
        list_categories(&cl);
        process::exit(1);
    }
    for c in &cl {
        if params.category_name == c.name {
            category = c.clone();
        }
    }

    if category.id == -1 {
        println!(
            "Category {} not found. Please use one of the following:\n",
            params.category_name
        );
        list_categories(&cl);
        process::exit(1);
    }

    // Now we know that the category specified exists in the database also check if
    // it exists in the configuration file
    let config_file_categories = match &settings.category {
        Some(c) => c,
        _ => {
            println!("No categories found in the configuration file");
            process::exit(1);
        }
    };

    let mut config_file_category = settings::Category::default();

    for c in config_file_categories {
        if c.name == params.category_name {
            config_file_category = c.clone();
        }
    }

    if config_file_category.name.is_empty() {
        println!(
            "Category '{}' not found in configuration file",
            params.category_name
        );
        process::exit(1);
    }

    let category_rsync_options: Vec<String> = match &config_file_category.options {
        Some(opt) => opt.split(' ').map(str::to_string).collect::<Vec<String>>(),
        _ => vec![],
    };

    let rsync_options: Vec<String> = match &settings.common_rsync_options {
        Some(ro) => ro.split(' ').map(str::to_string).collect::<Vec<String>>(),
        _ => vec![],
    };

    let mut excludes: Vec<String> = match &settings.excludes {
        Some(ex) => ex.to_vec(),
        _ => vec![],
    };

    let category_excludes: Vec<String> = match &config_file_category.excludes {
        Some(ex) => ex.to_vec(),
        _ => vec![],
    };

    excludes.extend(category_excludes);

    let mut cds: HashMap<String, CategoryDirectory> = HashMap::new();

    let topdir = match category.topdir.ends_with('/') {
        true => String::from(&category.topdir),
        false if category.topdir.is_empty() => String::from(""),
        false => format!("{}/", category.topdir),
    };

    if let Err(e) = match config_file_category.r#type.as_str() {
        "rsync" => scan_with_rsync(
            &mut cds,
            &excludes,
            &topdir,
            &category_rsync_options,
            &rsync_options,
            &config_file_category.url,
        ),
        "directory" => scan_local_directory(
            &mut cds,
            &excludes,
            &topdir,
            &config_file_category.url,
            params.skip_fftl,
        ),
        _ => {
            println!(
                "Cannot handle type '{}' of category '{}'",
                config_file_category.r#type, params.category_name
            );
            process::exit(1);
        }
    } {
        println!("Scanning {} failed with {}", &config_file_category.url, e);
        process::exit(1);
    }

    handle_unreadable(&mut cds);

    let mut d = db::functions::get_directories(&connection, category.id);

    if let Err(e) =
        sync_category_directories(&connection, topdir.clone(), category.id, &mut d, &mut cds)
    {
        println!("Syncing changes to database failed {}", e);
        process::exit(1);
    }

    let repositories = match db::functions::get_repositories(&connection) {
        Ok(r) => r,
        Err(e) => {
            println!("Reading repositories from the database failed: {:#?}", e);
            process::exit(1);
        }
    };
    let repository_mappings: Vec<settings::RepositoryMapping> = match &settings.repository_mapping {
        Some(rm) => rm.to_vec(),
        _ => Vec::new(),
    };

    let repository_aliases: Vec<settings::RepositoryAlias> = match &settings.repository_aliases {
        Some(ra) => ra.to_vec(),
        _ => Vec::new(),
    };

    let skip_paths: Vec<String> = match &settings.skip_paths_for_version {
        Some(ex) => ex.to_vec(),
        _ => vec![],
    };
    let test_paths: Vec<String> = match &settings.test_paths {
        Some(ex) => ex.to_vec(),
        _ => vec![],
    };
    let skip_repository_paths: Vec<String> = match &settings.skip_repository_paths {
        Some(ex) => ex.to_vec(),
        _ => vec![],
    };
    let do_not_display_paths: Vec<String> = match &settings.do_not_display_paths {
        Some(ex) => ex.to_vec(),
        _ => vec![],
    };
    let mut fds = db::functions::get_file_details(&connection);
    let mut find_parameter = FindRepositories {
        c: &connection,
        cds: &mut cds,
        checksum_base: match config_file_category.r#type.as_str() {
            "rsync" => config_file_category.checksum_base,
            "directory" => Some(
                config_file_category
                    .url
                    .split(&topdir)
                    .map(str::to_string)
                    .collect::<Vec<String>>()[0]
                    .clone(),
            ),
            _ => None,
        },
        top: topdir.clone(),
        cat: &category,
        repos: &repositories,
        rms: &repository_mappings,
        fds: &mut fds,
        skip_paths: &skip_paths,
        test_paths: &test_paths,
        skip_repository_paths: &skip_repository_paths,
        do_not_display_paths: &do_not_display_paths,
        backend: config_file_category.r#type,
        aliases: &repository_aliases,
    };
    if let Err(e) = find_repositories(&mut find_parameter) {
        println!("Creating repositories in database failed {}", e);
        process::exit(1);
    }

    if let Err(e) = age_file_details(
        &connection,
        &mut fds,
        &d,
        match settings.max_stale_days {
            Some(m) => m,
            _ => 3,
        },
        match settings.max_propagation_days {
            Some(m) => m,
            _ => 2,
        },
    ) {
        println!("File Detail aging failed {}", e);
        process::exit(1);
    }

    if params.delete_directories {
        if let Err(e) = cleanup_database(&connection, &cds, &d, topdir) {
            println!("Database cleanup failed {}", e);
            process::exit(1);
        }
    }
}

#[cfg(test)]
mod scan_primary_mirror_test;
