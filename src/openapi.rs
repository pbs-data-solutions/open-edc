use utoipa::OpenApi;

use crate::{api, models};

#[derive(OpenApi)]
#[openapi(
    paths(
        api::v1::routes::organization::create_organization,
        api::v1::routes::organization::delete_organization,
        api::v1::routes::organization::get_organization,
        api::v1::routes::organization::get_organizations,
        api::v1::routes::organization::update_organization,
        api::v1::routes::user::create_user,
        api::v1::routes::user::delete_user,
        api::v1::routes::user::get_user,
        api::v1::routes::user::get_users,
        api::v1::routes::user::update_user,
    ),
    components(schemas(
        models::organization::Organization,
        models::organization::OrganizationCreate,
        models::organization::OrganizationUpdate,
        models::messages::GenericMessage,
        models::user::User,
        models::user::UserCreate,
        models::user::UserUpdate,
    )),
    tags(
        (name = "Organization", description = "Organization management"),
        (name = "Users", description = "User managmenet"),
    ),
)]
pub struct ApiDoc;
