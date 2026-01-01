mod send;
mod utils;
mod config;
mod smtp;
mod imap;

use axum::{routing::get, extract::Form, Router};
use serde::Deserialize;
use tokio::sync::mpsc;

use send::send::SendInfo;
use smtp::SmtpMailer;
use imap::ImapListener;
use config::CONFIG;

async fn hello_world() -> &'static str {
    "Hello Rust! 🚀"
}

#[derive(Deserialize)]
struct WebHook2EmailParams {
    #[serde(rename = "title")]
    title: Option<String>,
    #[serde(rename = "message")]
    message: Option<String>,
    #[serde(rename = "sendTo")]
    send_to: Option<String>,
    #[serde(rename = "action-url")]
    action_url: Option<String>,
}

async fn web_hook_2_email(Form(params): Form<WebHook2EmailParams>) -> String {
    // 验证必填参数
    if [&params.title, &params.message, &params.send_to]
        .iter()
        .any(|field| field.is_none())
    {
        return "发送失败: 缺少必填参数".to_string();
    }

    let send_to_email = params.send_to.as_ref().unwrap();
    let message = params.message.unwrap();
    let title = params.title.unwrap();

    let mut send_info = SendInfo::default();
    send_info.user_email = CONFIG.smtp_email.clone();
    send_info.user = CONFIG.smtp_user.clone();
    send_info.send_to_email = send_to_email.to_string();
    send_info.send_to_user = "hezhihu89".to_string();
    send_info.send_data = message;
    send_info.send_title = title;
    send_info.from_email = "HomeStation Notification".to_string();
    send_info.foward_url = params
        .action_url
        .unwrap_or_else(|| utils::regex::extract_first_url(&send_info.send_data, "https://fn.osfile.cn"));

    println!("URL: {}", send_info.foward_url);

    let start = chrono::Local::now().timestamp_millis();
    let mailer = SmtpMailer::new();

    match mailer.send(send_info) {
        Ok(_) => {
            println!("send email to {} success", send_to_email);
            format!(
                "发送成功 耗时: {} ms",
                chrono::Local::now().timestamp_millis() - start
            )
        }
        Err(e) => {
            eprintln!("send email to {} failed: {}", send_to_email, e);
            format!(
                "发送失败: {} 耗时: {} ms",
                e,
                chrono::Local::now().timestamp_millis() - start
            )
        }
    }
}

/// 处理 IMAP 邮件事件的异步任务
async fn handle_imap_events(mut rx: mpsc::Receiver<SendInfo>) {
    println!("✅ 邮件处理器已启动");
    let mailer = SmtpMailer::new();
    while let Some(send_info) = rx.recv().await {
        println!("📨 收到待转发邮件: {}", send_info.send_title);
        match mailer.send(send_info) {
            Ok(_) => println!("✓ 转发成功"),
            Err(e) => eprintln!("❌ 转发失败: {}", e),
        }
    }
    println!("❌ handle_imap_events 任务结束");
}

#[tokio::main]
async fn main() {
    // 创建 Channel 用于 IMAP 和 SMTP 之间的通信
    let (tx, rx) = mpsc::channel::<SendInfo>(32);

    // 启动 Web 服务器
    let app = Router::new()
        .route("/", get(hello_world))
        .route("/webhook2email", get(web_hook_2_email).post(web_hook_2_email));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Failed to bind to 0.0.0.0:3000");
    let serve = axum::serve(listener, app);

    // 启动 IMAP 监听器
    let imap_listener = ImapListener::new();
    let imap_handler = tokio::spawn(imap_listener.start(tx));

    // 启动邮件转发处理器
    let email_handler = tokio::spawn(handle_imap_events(rx));

    // tokio::select! {
    //     _ = imap_handler => {
    //         println!("IMAP 服务已经停止");
    //     }
    //     _ = serve => {
    //         println!("WebServer 服务已经停止");
    //     }
    //     _ = email_handler => {
    //         println!("Email 处理器已经停止");
    //     }
    // }
     // 等待所有任务完成
    let join_result = tokio::join!(imap_handler, serve, email_handler);
    println!("所有服务已停止: {:?}", join_result);
}
