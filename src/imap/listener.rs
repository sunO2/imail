use imap::types::UnsolicitedResponse;
use mail_parser::MessageParser;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::config::CONFIG;
use crate::send::send::SendInfo;

/// IMAP 邮件监听器
pub struct ImapListener;

impl ImapListener {
    /// 创建新的 ImapListener 实例
    pub fn new() -> Self {
        Self
    }

    /// 连接并登录 IMAP 服务器
    fn connect_and_login(&self) -> imap::error::Result<imap::Session<Box<dyn imap::ImapConnection>>> {
        println!("Connecting to {}:993...", CONFIG.imap_server);
        let client = imap::ClientBuilder::new(&CONFIG.imap_server, 993).connect()?;

        println!("Logging in as {}...", CONFIG.imap_email);
        let imap_session = client
            .login(&CONFIG.imap_email, &CONFIG.email_password)
            .map_err(|(e, _)| e)?;

        Ok(imap_session)
    }

    /// 单次 IDLE 监听（复用同一个 session）
    fn idle_listen(
        &self,
        session: &mut imap::Session<Box<dyn imap::ImapConnection>>,
        tx: &mpsc::Sender<SendInfo>,
    ) -> imap::error::Result<()> {
        println!("Selecting INBOX...");
        session.select("INBOX")?;

        println!("🔔 Starting IDLE mode, waiting for new emails...");
        let mut num_responses = 0;

        // 设置 IDLE 超时时间（29分钟，避免服务器超时断开）
        let idle_result = session
            .idle()
            .timeout(Duration::from_secs(29 * 60))
            .wait_while(|response| {
                num_responses += 1;
                let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");

                match response {
                    UnsolicitedResponse::Exists(count) => {
                        println!(
                            "[{}] 📧 New email detected! Total messages: {}",
                            timestamp, count
                        );
                        return false;
                    }
                    UnsolicitedResponse::Recent(count) => {
                        println!("[{}] 📬 Recent messages: {}", timestamp, count);
                    }
                    UnsolicitedResponse::Expunge(seq) => {
                        println!("[{}] 🗑️  Message {} deleted", timestamp, seq);
                    }
                    _ => {
                        println!(
                            "[{}] IDLE response #{}: {:?}",
                            timestamp, num_responses, response
                        );
                    }
                }

                true
            });

        match idle_result {
            Ok(_) => {
                if let Some(send_info) = self.fetch_latest_email(session) {
                    println!("✓ Email fetched successfully");
                    // 在 blocking 线程中，可以使用 blocking_send
                    println!("📤 正在发送消息到通道");
                    match tx.try_send(send_info) {
                        Ok(_) => {
                            println!("✅ Email event sent to channel successfully");
                        }
                        Err(e) => {
                            eprintln!("❌ Failed to send email event: {:?}", e);
                        }
                    }
                } else {
                    eprintln!("❌ Failed to fetch email");
                };
                Ok(())
            }
            Err(e) => {
                eprintln!("❌ IDLE error: {:?}", e);
                Err(e)
            }
        }
    }

    /// 获取最新邮件
    fn fetch_latest_email(
        &self,
        session: &mut imap::Session<Box<dyn imap::ImapConnection>>,
    ) -> Option<SendInfo> {
        let mut send_info = SendInfo::default();
        send_info.user_email = CONFIG.imap_email.clone();
        send_info.user = CONFIG.imap_user.clone();
        send_info.send_to_email = "354137379@qq.com".to_string();
        send_info.send_to_user = "hezhihu89".to_string();

        // 获取邮箱状态
        let mailbox = session.examine("INBOX").ok()?;
        let exists = mailbox.exists;

        if exists > 0 {
            println!("Fetching latest email (message #{})...", exists);

            // 获取最新的邮件
            let messages = session.fetch(exists.to_string(), "RFC822").ok()?;

            // 解析邮件内容
            let body_text = messages
                .iter()
                .next()?
                .body()
                .map(|body| String::from_utf8_lossy(body).into_owned())
                .unwrap_or_default();

            // 解析邮件
            if let Some(message) = MessageParser::default().parse(body_text.as_bytes()) {
                let subject = message.subject().unwrap_or("无主题");
                println!("邮件标题: {}", subject);
                send_info.send_title = subject.to_string();

                let body_text = message.body_text(0).unwrap_or(std::borrow::Cow::Borrowed(""));
                println!("邮件内容: {}", body_text);
                send_info.send_data = body_text.to_string();

                let from_email = message
                    .from()
                    .and_then(|from| from.first())
                    .and_then(|f| f.address())
                    .unwrap_or("未知发件人");
                println!("发件人: <{}>", from_email);
                send_info.from_email = from_email.to_string();
            } else {
                return None;
            }
        }

        Some(send_info)
    }

    /// 启动监听任务（异步）
    pub async fn start(self, tx: mpsc::Sender<SendInfo>) {
        // 在独立的 blocking 线程中运行整个 IMAP 监听逻辑
        tokio::task::spawn_blocking(move || {
            // 只在启动时连接和登录一次
            match self.connect_and_login() {
                Ok(mut session) => {
                    println!("✓ Successfully connected and logged in");

                    // 在同一个连接中循环监听
                    loop {
                        match self.idle_listen(&mut session, &tx) {
                            Ok(_) => {
                                println!("IDLE session ended, restarting...");
                            }
                            Err(e) => {
                                eprintln!("❌ IDLE error: {:?}", e);
                                println!("Connection lost, reconnecting in 5 seconds...");
                                std::thread::sleep(Duration::from_secs(5));

                                // 连接断开，需要重新登录
                                match self.connect_and_login() {
                                    Ok(new_session) => {
                                        session = new_session;
                                        println!("✓ Reconnected successfully");
                                    }
                                    Err(e) => {
                                        eprintln!("❌ Reconnection failed: {:?}", e);
                                        std::thread::sleep(Duration::from_secs(5));
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
        });
    }
}

impl Default for ImapListener {
    fn default() -> Self {
        Self::new()
    }
}
