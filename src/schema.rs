// @generated automatically by Diesel CLI.

diesel::table! {
    fics (id) {
        id -> Text,
        name -> Text,
        url -> Text,
        last_updated -> Text,
        version -> Int4,
        description -> Text,
        authors -> Array<Nullable<Text>>,
        fandom -> Array<Nullable<Text>>,
        ship_type -> Array<Nullable<Text>>,
        language -> Nullable<Text>,
        chapters -> Nullable<Text>,
        kudos -> Nullable<Int4>,
        words -> Nullable<Int4>,
        series -> Nullable<Array<Nullable<Text>>>,
        hits -> Nullable<Int4>,
        merged_tags -> Nullable<Array<Nullable<Text>>>,
    }
}

diesel::table! {
    fics_tags (fic_id, tag_name) {
        fic_id -> Text,
        tag_name -> Text,
    }
}

diesel::table! {
    tags (name) {
        name -> Text,
        #[sql_name = "type"]
        type_ -> Text,
        parent -> Nullable<Text>,
        sibligs -> Nullable<Array<Nullable<Text>>>,
        children -> Nullable<Array<Nullable<Text>>>,
    }
}

diesel::joinable!(fics_tags -> fics (fic_id));
diesel::joinable!(fics_tags -> tags (tag_name));

diesel::allow_tables_to_appear_in_same_query!(fics, fics_tags, tags,);
