use axum::{
    extract::{Path, Query, State},
    Json,
};
use crate::api::AppState;
use crate::api::v4::extractors::MmAuthUser;
use crate::error::ApiResult;
use crate::mattermost_compat::models as mm;
use crate::api::v4::threads::{
    ThreadsPath, ThreadPath, ThreadsQuery,
    get_threads_internal, get_thread_internal,
    mark_all_read_internal, follow_thread_internal, unfollow_thread_internal
};

pub async fn get_threads(
    state: State<AppState>,
    auth: MmAuthUser,
    path: Path<ThreadsPath>,
    query: Query<ThreadsQuery>,
) -> ApiResult<Json<mm::ThreadResponse>> {
    get_threads_internal(state, auth, path, query).await
}

pub async fn mark_all_read(
    state: State<AppState>,
    auth: MmAuthUser,
    path: Path<ThreadsPath>,
) -> ApiResult<Json<serde_json::Value>> {
    mark_all_read_internal(state, auth, path).await
}

pub async fn get_thread(
    state: State<AppState>,
    auth: MmAuthUser,
    path: Path<ThreadPath>,
) -> ApiResult<Json<mm::Thread>> {
    get_thread_internal(state, auth, path).await
}

pub async fn follow_thread(
    state: State<AppState>,
    auth: MmAuthUser,
    path: Path<ThreadPath>,
) -> ApiResult<Json<mm::Thread>> {
    follow_thread_internal(state, auth, path).await
}

pub async fn unfollow_thread(
    state: State<AppState>,
    auth: MmAuthUser,
    path: Path<ThreadPath>,
) -> ApiResult<Json<mm::Thread>> {
    unfollow_thread_internal(state, auth, path).await
}
