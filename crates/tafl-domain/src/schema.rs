// @generated automatically by Diesel CLI.

diesel::table! {
    users (id) {
        id -> Integer,
        google_id -> Text,
        email -> Text,
        name -> Text,
        avatar_url -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}
