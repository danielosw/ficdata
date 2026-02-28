use diesel::prelude::*;
#[derive(Queryable, Selectable, Identifiable, PartialEq, Debug)]
#[diesel(table_name = crate::schema::fics)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct fics {
    pub id: String,
    pub name: String,
    pub url: String,
    pub last_updated: String,
    pub version: i32,
    pub description: String,
    pub authors: Vec<Option<String>>,
    pub fandom: Vec<Option<String>>,
    pub ship_type: Vec<Option<String>>,
    pub language: Option<String>,
    pub chapters: Option<String>,
    pub kudos: Option<i32>,
    pub words: Option<i32>,
    pub series: Option<Vec<Option<String>>>,
    pub hits: Option<i32>,
    pub merged_tags: Option<Vec<Option<String>>>,
}
#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = crate::schema::tags)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct tags {
    pub name: String,
    pub type_: String,
    pub parent: Option<String>,
    pub sibligs: Option<Vec<Option<String>>>,
    pub children: Option<Vec<Option<String>>>,
}
#[derive( Selectable, Queryable, Associations, Debug)]
#[diesel(table_name = crate::schema::fics_tags)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(belongs_to(fics, foreign_key = fic_id))]
#[diesel(belongs_to(tags, foreign_key = tag_name))]
pub struct fics_tags {
    pub fic_id: String,
    pub tag_name: String,
}