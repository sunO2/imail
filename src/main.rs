extern crate imap;

mod send;
mod utils;
mod config;

use askama::Template;
use axum::{
    routing::{get},
    Router,
    extract::Form,
};
use serde::Deserialize;
use mail_parser::*;

use send::send::SendInfo;
use std::time::Duration;
use lettre::message::{Mailbox, header::ContentType};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};

use crate::config::{CONFIG};


async fn imap_task(){
// 只在启动时连接和登录一次
    match connect_and_login() {
        Ok(mut session) => {
            println!("✓ Successfully connected and logged in");
            
            // 在同一个连接中循环监听
            loop {
                match idle_listen(&mut session) {
                    Ok(_) => {
                        println!("IDLE session ended, restarting...");
                        // IDLE 正常结束，继续下一轮
                    }
                    Err(e) => {
                        eprintln!("❌ IDLE error: {:?}", e);
                        println!("Connection lost, reconnecting in 5 seconds...");
                        std::thread::sleep(Duration::from_secs(5));
                        
                        // 连接断开，需要重新登录
                        match connect_and_login() {
                            Ok(new_session) => {
                                session = new_session;
                                println!("✓ Reconnected successfully");
                            }
                            Err(e) => {
                                eprintln!("❌ Reconnection failed: {:?}", e);
                                std::thread::sleep(Duration::from_secs(5));
                                // 继续尝试重连
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("❌ Initial connection failed: {:?}", e);
            eprintln!("Please check your credentials and network connection");
        }
    }
}


async fn hello_world() -> &'static str {
    "Hello Rust! 🚀"
}

#[derive(Deserialize)]
struct WebHook2EmailParams {
    #[serde(rename = "title")]
    title: Option<String>,  //
    #[serde(rename = "message")] 
    message: Option<String>,  //
    #[serde(rename = "sendTo")] 
    send_to: Option<String>,  //
    #[serde(rename = "action-url")] 
    action_url: Option<String>,  // 可选参数
}
async fn web_hook_2_email(Form(params): Form<WebHook2EmailParams>) -> String {

    if [&params.title,&params.message,&params.send_to].iter().any(|field|field.is_none()){
        return format!("发送失败 没有消息信息")
    }

    let mut send_info = SendInfo::default();
    send_info.user_email = CONFIG.smtp_email.clone();
    send_info.user = CONFIG.smtp_user.clone();
    send_info.send_to_email = String::from("354137379@qq.com");
    send_info.send_to_user = String::from("hezhihu89");
    send_info.send_data = params.message.unwrap();
    send_info.send_title = params.title.unwrap();
    send_info.from_email = "HomeStation Notification".to_string();
    send_info.foward_url = params.action_url.unwrap_or(utils::regex::extract_first_url(&send_info.send_data, "https://fn.osfile.cn"));
    println!("url 获取到了{}",send_info.foward_url);
    let start = chrono::Local::now().timestamp_millis();
    match send_email_to_main(send_info) {
        Ok(_) => {
            println!("send email to {} success",params.send_to.unwrap());
            return format!("发送成功 耗时: {} ms", (chrono::Local::now().timestamp_millis() - start));
        },
        Err(_) => {
            println!("send email to {} faile",params.send_to.unwrap());
             return format!("发送成功 耗时: {} ms", (chrono::Local::now().timestamp_millis() - start));
        }
    }
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(hello_world))
        .route("/webhook2email", get(web_hook_2_email).post(web_hook_2_email));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    let serve = axum::serve(listener, app);

    let imap_handler = tokio::spawn(imap_task());
    tokio::select! {
        _ = imap_handler => {
            println!("IMAP 服务已经停止")
        },
        _ = serve => {
             println!("WebServe 服务已经停止")
        }
    }
}

// 连接和登录（只在需要时调用）
fn connect_and_login() -> imap::error::Result<imap::Session<Box<dyn imap::ImapConnection>>> {
    println!("Connecting to {}:993...",CONFIG.imap_server);
    let client = imap::ClientBuilder::new(&CONFIG.imap_server, 993).connect()?;
    
    println!("Logging in as {}...",CONFIG.imap_email);
    let imap_session = client
        .login(&CONFIG.imap_email, &CONFIG.email_password)
        .map_err(|(e, _)| e)?;
    
    Ok(imap_session)
}

// 单次 IDLE 监听（复用同一个 session）
fn idle_listen(session: &mut imap::Session<Box<dyn imap::ImapConnection>>) -> imap::error::Result<()> {
    println!("Selecting INBOX...");
    session.select("INBOX")?;
    
    println!("🔔 Starting IDLE mode, waiting for new emails...");
    let mut num_responses = 0;
    
    // 设置 IDLE 超时时间（29分钟，避免服务器超时断开）
    let idle_result = session.idle()
        .timeout(Duration::from_secs(29 * 60))
        .wait_while(|response| {
            num_responses += 1;
            let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
            
            // 检测到新邮件时的处理
            match response {
                imap::types::UnsolicitedResponse::Exists(count) => {
                    println!("[{}] 📧 New email detected! Total messages: {}", timestamp, count);
                    // 这里可以获取新邮件详情
                    // 注意：在 IDLE 的 wait_while 回调中无法直接操作 session
                    // 需要返回 false 退出 IDLE，然后在外部处理
                    return false
                }
                imap::types::UnsolicitedResponse::Recent(count) => {
                    println!("[{}] 📬 Recent messages: {}", timestamp, count);
                }
                imap::types::UnsolicitedResponse::Expunge(seq) => {
                    println!("[{}] 🗑️  Message {} deleted", timestamp, seq);
                }
                _ => {
                    println!("[{}] IDLE response #{}: {:?}", timestamp, num_responses, response);
                }
            }
            
            
            // 返回 true 继续监听
            // 如果想在检测到新邮件时退出 IDLE 去获取邮件，可以返回 false
            true
        });
    
    match idle_result {
        Ok(reason) => {
            if let Some(send_info) = fetch_latest_email(session) {
                    println!("✓ Email fetched successfully\n");
                    match send_email_to_main(send_info){
                        Ok(_) => {
                            println!("转发成功")
                        },
                        Err(e) => {
                            println!("发送失败了: {}",e)
                        }
                    }
            } else {
                eprintln!("❌ Failed to fetch email:")
            };
            println!("IDLE finished: {:?}", reason);
            Ok(())
        }
        Err(e) => {
            eprintln!("IDLE error: {:?}", e);
            Err(e)
        }
    }
}

// 获取最新邮件的辅助函数
#[allow(dead_code)]
fn fetch_latest_email(session: &mut imap::Session<Box<dyn imap::ImapConnection>>) -> Option<send::send::SendInfo> {
    let mut send_info = SendInfo::default();
    send_info.user_email = String::from(CONFIG.imap_email.clone());
    send_info.user = String::from(CONFIG.imap_user.clone());
    send_info.send_to_email = String::from("354137379@qq.com");
    send_info.send_to_user = String::from("hezhihu89");
  
    // 获取邮箱状态
    let mailbox = session.examine("INBOX").ok()?;
    
    let exists = mailbox.exists;
    if exists > 0 {
        println!("Fetching latest email (message #{})...", exists);
            // 获取最新的邮件
            let messages = session.fetch(exists.to_string(), "RFC822").ok()?;
            
            let body_text = if let Some(message) = messages.iter().next() {
                 let a =  message
                    .body()
                    .map(|body| String::from_utf8(body.to_vec()).expect("message was not valid utf-8"))
                      .unwrap_or_else(String::new);
                a
            } else {
                return None;
            };

            if let Some(message) = MessageParser::default().parse(body_text.as_bytes()) {
                let subject = message.subject().unwrap();
                println!("邮件标题: subject {}",subject);
                send_info.send_title = subject.to_string();
                let body_text = message.body_text(0).unwrap();
                println!("邮件内容: body_text {}",body_text);
                send_info.send_data = body_text.to_string();
                let from_email = if let Some(from) = message.from() {
                    let email = if let Some(first) = from.first() {
                        // let name = first.name().unwrap_or("");
                        let email = first.address().unwrap_or("unknown");
                        email
                    } else {
                        ""
                    };
                    email
                }else {
                    ""
                };
                println!("发件人:  <{}>", from_email);
                send_info.from_email = from_email.to_string();
                }else {
                    return None;
                }
        }
    
    Some(send_info)
}


/** 发送邮件到指定邮箱 */
fn send_email_to_main(send_info: send::send::SendInfo) -> Result<<SmtpTransport as Transport>::Ok, <SmtpTransport as Transport>::Error> {
       
       let smtp_send_template = SmtpSendTemplate {
            title: send_info.send_title.clone(),
            from: send_info.from_email,
            action_url: send_info.foward_url,
            message: send_info.send_data,
       };
       let send_html_text = match smtp_send_template.render() {
        Ok(html) => {
            html
        },
        Err(e) => {
            println!("转换错误 {}",e);
            "".to_string()
        }
       };

       let email = Message::builder()
        .from(Mailbox::new(Some(send_info.user.to_owned()), send_info.user_email.parse().unwrap()))
        // .reply_to(Mailbox::new(Some("Yuin".to_owned()), "yuin@domain.tld".parse().unwrap()))
        .to(Mailbox::new(Some(send_info.send_to_user.to_owned()), send_info.send_to_email.parse().unwrap()))
        .subject(send_info.send_title)
        .header(ContentType::TEXT_HTML)
        .body(send_html_text)
        .unwrap();

    let creds = Credentials::new(CONFIG.smtp_email.clone(), CONFIG.email_password.clone());

    // Open a remote connection to gmail
    let mailer = SmtpTransport::relay(CONFIG.smtp_server.as_str())
        .unwrap()
        .credentials(creds)
        .build();

    // Send the email
    return mailer.send(&email)
}


#[derive(Template)]
#[template(path = "email.html")]
struct SmtpSendTemplate {
    title: String,
    from: String,
    action_url: String,
    message: String,
}
