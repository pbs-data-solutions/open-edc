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
        api::v1::routes::study::create_study,
        api::v1::routes::study::delete_study,
        api::v1::routes::study::get_studies,
        api::v1::routes::study::get_study,
        api::v1::routes::study::update_study,
        api::v1::routes::user::create_user,
        api::v1::routes::user::delete_user,
        api::v1::routes::user::get_user,
        api::v1::routes::user::get_users,
        api::v1::routes::user::update_user,
        api::v1::routes::user::user_add_study,
        api::v1::routes::user::user_remove_study,
    ),
    components(schemas(
        models::messages::GenericMessage,
        models::organization::Organization,
        models::organization::OrganizationCreate,
        models::organization::OrganizationUpdate,
        models::study::Study,
        models::study::StudyCreate,
        models::study::StudyUpdate,
        models::user::User,
        models::user::UserCreate,
        models::user::UserStudy,
        models::user::UserUpdate,
    )),
    tags(
        (name = "Organizations", description = "Organization management"),
        (name = "Studies", description = "Study management"),
        (name = "Users", description = "User managmenet"),
    ),
)]
pub struct ApiDoc;
