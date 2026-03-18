mod common;

use site::Site;
use std::env;

#[test]
fn errors_on_index_md_page_in_section() {
    let mut path = env::current_dir().unwrap().parent().unwrap().parent().unwrap().to_path_buf();
    path.push("test_sites_invalid");
    path.push("indexmd");
    let config_file = path.join("config.toml");
    let mut site = Site::new(&path, &config_file).unwrap();
    let res = site.load();
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(
        format!("{:?}", err).contains(
            "We can't have a page called `index.md` in the same folder as an index section"
        )
    );
}

#[test]
fn errors_on_duplicate_answer_ids() {
    let mut path = env::current_dir().unwrap().parent().unwrap().parent().unwrap().to_path_buf();
    path.push("test_sites_invalid");
    path.push("answers_duplicate_id");
    let config_file = path.join("config.toml");
    let mut site = Site::new(&path, &config_file).unwrap();
    let res = site.load();
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(err.to_string().contains("Duplicate answer id `duplicate-answer`"));
}

#[test]
fn errors_on_unknown_related_answer_ids() {
    let mut path = env::current_dir().unwrap().parent().unwrap().parent().unwrap().to_path_buf();
    path.push("test_sites_invalid");
    path.push("answers_missing_related");
    let config_file = path.join("config.toml");
    let mut site = Site::new(&path, &config_file).unwrap();
    let res = site.load();
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(err.to_string().contains("`related` references unknown answer id `missing-answer`"));
}
