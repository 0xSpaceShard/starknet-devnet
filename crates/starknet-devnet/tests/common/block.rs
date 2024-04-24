use starknet_rs_core::types::BlockTag;

pub struct TestBlockTag(pub BlockTag);

impl std::fmt::Display for TestBlockTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match &self {
            TestBlockTag(BlockTag::Latest) => "latest",
            TestBlockTag(BlockTag::Pending) => "pending",
        })
    }
}
