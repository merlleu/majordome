use majordome_scylla::{ScyllaORMTable, ScyllaRow};
use scylla::FromRow;

#[derive(ScyllaRow, FromRow)]
#[majordome_scylla(table = "users", primary_key = "id", indexes= "email")]
pub struct UserDBRepr {
    pub id: i64,                 // pk=static
    pub email: Option<String>,   // string = set
    pub sponsor_id: Option<i64>, // int = set
    pub p_desc: Option<String>,  // string = set
    #[majordome_scylla(map = 1)]
    pub assets: std::collections::BTreeMap<String, String>, // map = set,mapadd,mapremove
    pub flags: i64,
}

#[test]
fn test() {
    use std::collections::BTreeMap;

    let u = UserDBRepr::new(1);
    let mut m = u.update();
    m.email_set(Some("test".to_string())).assets_add({
        let mut m = BTreeMap::new();
        m.insert("test".to_string(), "test".to_string());
        m
    });

    assert!(!m.is_saved())
}
