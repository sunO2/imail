
#[derive(Default)]
pub struct SendInfo {
    /**邮件发送人名称 */
    pub user: String,
    /** 邮件发送人 email */
    pub user_email: String,
    /** 邮件接收人名称 */
    pub send_to_user: String,
    /** 邮件接收人 email */
    pub send_to_email: String,

    /** 发送者邮箱 */
    pub from_email: String,
    /** 发送标题 */
    pub send_title: String,
    /** 发送消息内容 */
    pub send_data: String,
    pub foward_url: String,

}