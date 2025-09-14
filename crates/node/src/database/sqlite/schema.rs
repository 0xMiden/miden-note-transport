// @generated automatically by Diesel CLI.

diesel::table! {
    notes (id) {
        id -> Binary,
        tag -> BigInt,
        header -> Binary,
        details -> Binary,
        created_at -> BigInt,
    }
}
