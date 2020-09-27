/// NBT tag constants
pub mod tags {
    pub const TAG_LEVEL: &str = "Level";
    pub const TAG_X_POS: &str = "xPos";
    pub const TAG_Z_POS: &str = "zPos";
    pub const TAG_SECTIONS: &str = "Sections";
    pub const TAG_LAST_UPDATE: &str = "LastUpdate";
    pub const TAG_INHABITED_TIME: &str = "InhabitedTime";
    pub const TAG_HEIGHTMAPS: &str = "Heightmaps";
    pub const TAG_CARVING_MASKS: &str = "CarvingMasks";
    pub const TAG_ENTITIES: &str = "Entities";
    pub const TAG_TILE_ENTITIES: &str = "TileEntities";
    pub const TAG_TILE_TICKS: &str = "TileTicks";
    pub const TAG_LIQUID_TICKS: &str = "LiquidTicks";
    pub const TAG_LIGHTS: &str = "Lights";
    pub const TAG_LIQUIDS_TO_BE_TICKED: &str = "LiquidsToBeTicked";
    pub const TAG_TO_BE_TICKED: &str = "ToBeTicked";
    pub const TAG_POST_PROCESSING: &str = "PostProcessing";
    pub const TAG_STATUS: &str = "Status";
    pub const TAG_STRUCTURES: &str = "Structures";

    /// A list of required tags stored in the level tag
    pub const LEVEL_TAGS: &[&'static str] = &[
        TAG_X_POS,
        TAG_Z_POS,
        TAG_SECTIONS,
        TAG_LAST_UPDATE,
        TAG_INHABITED_TIME,
        TAG_HEIGHTMAPS,
        TAG_ENTITIES,
        TAG_TILE_ENTITIES,
        TAG_LIQUID_TICKS,
        TAG_POST_PROCESSING,
        TAG_STATUS,
        TAG_STRUCTURES,
    ];
}
