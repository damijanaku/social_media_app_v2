use crate::services::comment_service::{delete_comment, get_comments, post_comment};
use crate::services::like_service::{get_likes, like_post, unlike_post};
use crate::services::post_service::{
    create_post, delete_post, get_feed, get_my_posts, get_post_by_id, get_posts_from_user,
    update_post,
};
use crate::services::user_service::{
    delete_user, follow_user, get_followers, get_following, get_my_followers, get_my_following,
    get_user, get_user_by_username, login_user, my_profile, profile, register_user, unfollow_user,
    update_user,
};
use crate::utils::jwt::refresh_token;
use axum::{Router, routing::delete, routing::get, routing::post, routing::put};
use sqlx::PgPool;

pub fn create_router(pool: PgPool) -> Router<()> {
    Router::new()
        .route("/api/v1/auth/refresh", post(refresh_token))
        .route("/api/v1/users/login", post(login_user))
        .route("/api/v1/users/register", post(register_user))
        .route(
            "/api/v1/users/follow/followers/:target_id",
            get(get_followers),
        )
        .route(
            "/api/v1/users/follow/following/:target_id",
            get(get_following),
        )
        .route("/api/v1/users/me/followers", get(get_my_followers))
        .route("/api/v1/users/me/following", get(get_my_following))
        .route(
            "/api/v1/users/username/:username",
            get(get_user_by_username),
        )
        .route("/api/v1/users/follow/:target_id", post(follow_user))
        .route("/api/v1/users/unfollow/:target_id", post(unfollow_user))
        .route("/api/v1/users/profile", get(my_profile))
        .route("/api/v1/users/profile/:target_id", get(profile))
        .route("/api/v1/users/:target_id", get(get_user))
        .route("/api/v1/users/me", put(update_user).delete(delete_user))
        .route("/api/v1/posts", post(create_post))
        .route("/api/v1/posts/me", get(get_my_posts))
        .route("/api/v1/posts/feed", get(get_feed))
        .route("/api/v1/posts/user/:target_id", get(get_posts_from_user))
        .route(
            "/api/v1/posts/:target_id",
            get(get_post_by_id).put(update_post).delete(delete_post),
        )
        .route(
            "/api/v1/posts/comments/:target_id",
            get(get_comments).post(post_comment),
        )
        .route(
            "/api/v1/posts/comments/:target_id/:comment_id",
            delete(delete_comment),
        )
        .route(
            "/api/v1/posts/likes/:target_id",
            get(get_likes).post(like_post).delete(unlike_post),
        )
        .with_state(pool)
}
