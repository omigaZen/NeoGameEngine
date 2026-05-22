use crate::{handle::UntypedHandle, id::AssetId, loader::LoadPriority};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct StreamingRegionId(pub u64);

#[derive(Clone, Debug, PartialEq)]
pub struct StreamingRegion {
    pub id: StreamingRegionId,
    pub name: String,
    pub priority: LoadPriority,
    pub assets: Vec<UntypedHandle>,
    pub resident: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StreamingCommand {
    Preload(StreamingRegionId),
    Unload(StreamingRegionId),
    SetResident {
        region: StreamingRegionId,
        resident: bool,
    },
    AddAsset {
        region: StreamingRegionId,
        asset: AssetId,
    },
}
