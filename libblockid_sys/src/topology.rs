enum TopologyTag {
    LogicalSectorSize(u64),
    PhysicalSectorSize(u64),
    MinimumIoSize(u64),
    OptimalIoSize(u64),
    AlignmentOffset(Option<u64>),
}
