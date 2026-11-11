use tauri::http::{Request, Response};
use tauri::Manager;

use crate::{services, AppState};

/// 附件图片内联协议：`http://wudaores.localhost/{resource_id}`
/// 仅允许 image 类资源；路径解析在 Rust 端完成并做穿越校验。
pub fn handle_resource_protocol(
    app: &tauri::AppHandle,
    request: &Request<Vec<u8>>,
) -> Response<Vec<u8>> {
    let id = request
        .uri()
        .path()
        .trim_start_matches('/')
        .split(['?', '#'])
        .next()
        .unwrap_or("")
        .to_string();

    let not_found = |msg: &str| {
        Response::builder()
            .status(404)
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(msg.as_bytes().to_vec())
            .unwrap()
    };

    // UUID 格式预校验
    if uuid::Uuid::parse_str(&id).is_err() {
        return not_found("资源 ID 不合法");
    }

    let root = match app.path().app_local_data_dir() {
        Ok(dir) => services::resources::AttachmentRoot(dir.join("attachments")),
        Err(_) => return not_found("应用数据目录不可用"),
    };

    let (res, path) = {
        let state = app.state::<AppState>();
        let conn = match state.db.lock() {
            Ok(c) => c,
            Err(_) => return not_found("数据库被占用"),
        };
        match services::resources::get(&conn, &root, &id) {
            Ok(r) => {
                let path = match services::resources::resolve_file_path(&root, &r) {
                    Ok(p) => p,
                    Err(_) => return not_found("附件文件不存在"),
                };
                (r, path)
            }
            Err(_) => return not_found("资源不存在"),
        }
    };

    if !services::resources::is_inline_image(&res) {
        return not_found("该资源不支持内联预览");
    }
    if !path.is_file() {
        return not_found("文件不存在");
    }

    match std::fs::read(&path) {
        Ok(bytes) => {
            if bytes.len() as i64 > services::resources::MAX_INLINE_BYTES {
                return Response::builder()
                    .status(413)
                    .header("Content-Type", "text/plain; charset=utf-8")
                    .body("图片过大，不支持内联预览".as_bytes().to_vec())
                    .unwrap();
            }
            Response::builder()
                .status(200)
                .header("Content-Type", services::resources::mime_for(&res))
                .header("Cache-Control", "no-store")
                .body(bytes)
                .unwrap()
        }
        Err(_) => not_found("读取文件失败"),
    }
}
