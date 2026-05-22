fn main() {
    let report = asset_smoke::run_smoke();
    let editor = asset_smoke::run_editor_smoke();
    let model = asset_smoke::run_model_smoke();
    println!(
        "asset smoke render={} audio={} physics={} material={} group={}/{} ready_events={} dependency_events={} failed_events={}",
        report.render_ready,
        report.audio_ready,
        report.physics_ready,
        report.material_ready_with_dependencies,
        report.group_ready_assets,
        report.group_total_assets,
        report.ready_events,
        report.dependency_events,
        report.failed_events
    );
    println!(
        "asset editor smoke scanned={} imported={} cooked={} bundled={} bundle_ready={} material_ready={} ready_events={} failed_events={}",
        editor.scanned_sources,
        editor.imported_assets,
        editor.cooked_assets,
        editor.bundled_assets,
        editor.bundle_group_ready,
        editor.material_ready_with_dependencies,
        editor.ready_events,
        editor.failed_events
    );
    println!(
        "asset model smoke generated={} bundled={} bundle_ready={} mesh={} material={} skeleton={} animation={} material_dependencies={}",
        model.generated_subresources,
        model.bundled_assets,
        model.bundle_group_ready,
        model.mesh_ready,
        model.material_ready_with_dependencies,
        model.skeleton_ready,
        model.animation_ready,
        model.material_dependencies
    );
}
