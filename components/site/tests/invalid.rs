mod common;

use site::Site;

#[test]
fn errors_on_index_md_page_in_section() {
    let path = common::repo_path(&format!("{}/indexmd", common::INVALID_FIXTURES_ROOT));
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
    let path =
        common::repo_path(&format!("{}/answers_duplicate_id", common::INVALID_FIXTURES_ROOT));
    let config_file = path.join("config.toml");
    let mut site = Site::new(&path, &config_file).unwrap();
    let res = site.load();
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(err.to_string().contains("Duplicate answer id `duplicate-answer`"));
}

#[test]
fn errors_on_duplicate_canonical_questions() {
    let path =
        common::repo_path(&format!("{}/answers_duplicate_question", common::INVALID_FIXTURES_ROOT));
    let config_file = path.join("config.toml");
    let mut site = Site::new(&path, &config_file).unwrap();
    let res = site.load();
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(err.to_string().contains("Duplicate canonical question `how do refunds work`"));
}

#[test]
fn errors_on_unknown_related_answer_ids() {
    let path =
        common::repo_path(&format!("{}/answers_missing_related", common::INVALID_FIXTURES_ROOT));
    let config_file = path.join("config.toml");
    let mut site = Site::new(&path, &config_file).unwrap();
    let res = site.load();
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(err.to_string().contains("`related` references unknown answer id `missing-answer`"));
}

#[test]
fn errors_on_invalid_structured_data_sidecar() {
    let path = common::repo_path(&format!(
        "{}/answers_invalid_schema_sidecar",
        common::INVALID_FIXTURES_ROOT
    ));
    let config_file = path.join("config.toml");
    let mut site = Site::new(&path, &config_file).unwrap();
    let res = site.load();
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(err.to_string().contains("Failed to parse structured-data sidecar"));
}
