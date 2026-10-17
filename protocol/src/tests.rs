use super::*;
use prost::Message;

#[test]
fn friend_list_reply_decodes_the_friend_module_friend_info() {
    // info { userId: 7, name: "Sonetto" }; field 2 is a string in FriendModule's FriendInfo.
    let mut bytes = vec![0x0a, 0x0b, 0x08, 0x07, 0x12, 0x07];
    bytes.extend_from_slice(b"Sonetto");

    let reply = GetFriendInfoListReply::decode(&bytes[..]).expect("valid friend list reply");

    assert_eq!(reply.info[0].user_id, Some(7));
    assert_eq!(reply.info[0].name.as_deref(), Some("Sonetto"));
}
