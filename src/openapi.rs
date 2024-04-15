use utoipa::OpenApi;

use crate::{models, routes};

#[derive(OpenApi)]
#[openapi(
    paths(
        routes::organization::create_organization,
        routes::organization::delete_organization,
        routes::organization::get_organization,
        routes::organization::get_organizations,
        routes::organization::update_organization,
        routes::study::create_study,
        routes::study::delete_study,
        routes::study::get_studies,
        routes::study::get_study,
        routes::study::update_study,
        routes::user::create_user,
        routes::user::delete_user,
        routes::user::get_user,
        routes::user::get_users,
        routes::user::update_user,
        routes::user::user_add_study,
        routes::user::user_remove_study,
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
