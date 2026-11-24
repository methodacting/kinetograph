//! `<canvas>` with a WebGL context (packages/cli/skills/kinetograph-markup/references/SCENE.md, "glsl"): the recording the TS side puts in `data-kg-webgl` is one
//! glsl layer in the painted scene. Each case is a file of WebGL calls (`tests/webgl/<name>.js`) and the recording
//! the recorder made of it (`<name>.json`, `tools/webgl-cases.mjs`). The page has both: Chrome runs the calls with
//! its own WebGL (the oracle; SwiftShader, so the numbers do not depend on the GPU), and the engine runs the
//! recording with naga and wgpu. The two pictures are compared (they are in `target/paint-tests`), and ours
//! against its committed golden.

mod paintkit;

use kinetograph_core::{Frame, Renderer, Scene, render_still};
use paintkit::compare;

fn case_file(name: &str, extension: &str) -> String {
    let path = paintkit::repo_root()
        .join("crates/dom/tests/webgl")
        .join(format!("{name}.{extension}"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// The context attributes the case asks for on its second line (`// attributes: {...}`), as JSON.
fn attributes_of(script: &str) -> String {
    script
        .lines()
        .nth(1)
        .and_then(|l| l.strip_prefix("// attributes: "))
        .unwrap_or("{}")
        .to_string()
}

fn size_of(script: &str) -> (u32, u32) {
    let line = script.lines().next().unwrap_or_default();
    let mut numbers = line
        .strip_prefix("// size: ")
        .unwrap_or_else(|| panic!("the first line is the size, got {line:?}"))
        .split(' ')
        .map(|n| n.parse::<u32>().unwrap());
    (numbers.next().unwrap(), numbers.next().unwrap())
}

/// The mean and largest difference from Chrome (`None` without Chrome).
fn run(name: &str, mean_limit: f64) -> Option<(f64, u8)> {
    let script = case_file(name, "js");
    let recording = case_file(name, "json");
    let (w, h) = size_of(&script);
    assert!(
        !recording.contains('\''),
        "{name}.json has a single quote, which would end its attribute"
    );
    // GLSL ES 3.00 needs a WebGL 2 context.
    let ctx = if script.contains("#version 300 es") {
        "webgl2"
    } else {
        "webgl"
    };
    let attributes = attributes_of(&script);
    let body = format!(
        "<canvas id=\"c\" width=\"{w}\" height=\"{h}\" data-kg-webgl='{}' style=\"display:block\"></canvas>\n\
         <script>\n\
         const W = {w}, H = {h};\n\
         (function (gl, W, H) {{\n{script}\n}})(document.getElementById('c').getContext('{ctx}', {attributes}), W, H);\n\
         </script>",
        recording.trim()
    );
    let (out, frame, diff) = compare(name, &body, w, h);
    assert!(
        out.unsupported.is_empty(),
        "{name}: unsupported {:?}",
        out.unsupported
    );
    paintkit::check_golden(name, &frame);
    println!("{name}: {diff:?}");
    if let Some(d) = diff {
        assert!(d.mean < mean_limit, "{name}: {d:?}");
    }
    diff.map(|d| (d.mean, d.max))
}

#[test]
fn webgl_gradient() {
    run("webgl_gradient", 1.0);
}

#[test]
fn webgl_noise() {
    run("webgl_noise", 1.0);
}

#[test]
fn webgl_texture() {
    run("webgl_texture", 2.0);
    // One blob under three descriptions: 4 x 1 and 2 x 2 RGBA8, and 1 x 1 RGBA32F. Each third of the canvas shows its
    // own picture, so a texture on the device is its description, never its bytes alone.
    let recording = case_file("webgl_blob_descriptions", "json");
    let content: serde_json::Value = serde_json::from_str(&recording).unwrap();
    let blobs: std::collections::BTreeSet<&str> = content["textures"]
        .as_object()
        .unwrap()
        .values()
        .map(|t| t["data"]["blob"].as_str().unwrap())
        .collect();
    assert_eq!(blobs.len(), 1, "the three textures name one blob");
    let frame = paintkit::render_output(&painted(&recording, (48, 16)), 48, 16);
    let near = |x: usize, y: usize, want: [u8; 3]| {
        let got = px(&frame, x, y);
        assert!(
            (0..3).all(|c| got[c].abs_diff(want[c]) <= 1) && got[3] == 255,
            "({x}, {y}) is {got:?}, not {want:?}"
        );
    };
    // As 4 x 1: the four texels across.
    for (x, want) in [(2, [200, 0, 128]), (6, [0, 200, 0]), (10, [0, 0, 200]), (14, [0, 0, 0])] {
        near(x, 8, want);
    }
    // As 2 x 2: the first two texels are GL's bottom row.
    near(20, 12, [200, 0, 128]);
    near(28, 12, [0, 200, 0]);
    near(20, 4, [0, 0, 200]);
    near(28, 4, [0, 0, 0]);
    // As one texel of four floats, about (1.0, 0.503, 0.391).
    near(40, 8, [255, 128, 100]);
}

/// The page with the recording `recording` on a canvas of `(w, h)`, painted on a page of that size.
fn painted(recording: &str, (w, h): (u32, u32)) -> kinetograph_dom::request::IslandOutput {
    let body = format!(
        "<canvas width=\"{w}\" height=\"{h}\" data-kg-webgl='{}' style=\"display:block\"></canvas>",
        recording.trim()
    );
    paintkit::paint_html(&paintkit::page(&body), w, h)
}

/// The scene of `painted`, read as a render reads it (blobs relative to the pages' base directory).
fn scene_of(recording: &str, (w, h): (u32, u32)) -> kinetograph_core::Result<Scene> {
    let json = paintkit::scene_json(&painted(recording, (w, h)), w, h).to_string();
    Scene::from_json(&json, paintkit::repo_root().join("crates/dom/tests/assets"))
}

fn px(frame: &Frame, x: usize, y: usize) -> [u8; 4] {
    let i = (y * frame.width as usize + x) * 4;
    frame.rgba[i..i + 4].try_into().unwrap()
}

#[test]
fn a_later_frame_draws_the_target_it_carries_after_the_caches_forget_it() {
    // The recording of a frame that only samples a target, carrying the draw and the upload that filled it.
    let replay = case_file("webgl_blob_replay", "json");
    let content: serde_json::Value = serde_json::from_str(&replay).unwrap();
    let ops: Vec<&str> = content["ops"].as_array().unwrap().iter().map(|o| o["op"].as_str().unwrap()).collect();
    assert_eq!(ops, ["draw", "upload", "draw"]);
    let size = (48, 16);
    let mut renderer = Renderer::new(scene_of(&replay, size).unwrap()).unwrap();
    // Every payload but the one used last is forgotten as soon as another is kept.
    renderer.set_glsl_budget(1);
    let fresh = renderer.render_frame(0).unwrap();
    // The left texel is the picture's red, the right the upload's green.
    assert_eq!(px(&fresh, 4, 8), [255, 0, 0, 255]);
    assert_eq!(px(&fresh, 12, 8), [0, 255, 0, 255]);
    renderer
        .set_scene(scene_of(&case_file("webgl_blob_descriptions", "json"), size).unwrap())
        .unwrap();
    renderer.render_frame(0).unwrap();
    renderer.set_scene(scene_of(&replay, size).unwrap()).unwrap();
    let again = renderer.render_frame(0).unwrap();
    assert!(fresh.rgba == again.rgba, "the replay drew other pixels after the caches forgot it");
}

#[test]
fn a_blob_that_is_misnamed_or_missing_fails_the_recording_by_name() {
    let replay = case_file("webgl_blob_replay", "json");
    let content: serde_json::Value = serde_json::from_str(&replay).unwrap();
    let named = content["textures"]
        .as_object()
        .unwrap()
        .values()
        .find_map(|t| t["data"]["blob"].as_str())
        .unwrap();
    // Not sha256:<64 hex>: the scene is refused.
    let misnamed = "sha256:../../x";
    let e = scene_of(&replay.replace(named, misnamed), (16, 16)).unwrap_err().to_string();
    assert!(e.contains(misnamed), "{e}");
    // Well named, with no file: the frame fails, naming it.
    let missing = format!("sha256:{}", "0".repeat(64));
    let scene = scene_of(&replay.replace(named, &missing), (16, 16)).unwrap();
    let e = render_still(scene, 0).unwrap_err().to_string();
    assert!(e.contains(&missing), "{e}");
}

#[test]
fn webgl_framebuffer() {
    run("webgl_framebuffer", 1.0);
}

#[test]
fn webgl_stencil() {
    run("webgl_stencil", 1.0);
}

#[test]
fn webgl2_mrt() {
    run("webgl2_mrt", 1.0);
}

#[test]
fn webgl2_depth_texture() {
    run("webgl2_depth_texture", 1.0);
}

#[test]
fn webgl2_blit_depth_stencil() {
    run("webgl2_blit_depth_stencil", 1.0);
}

#[test]
fn webgl2_cube_rt() {
    run("webgl2_cube_rt", 1.0);
}

#[test]
fn webgl2_mipmap_after_draw() {
    run("webgl2_mipmap_after_draw", 1.0);
}

#[test]
fn webgl2_msaa_resolve() {
    run("webgl2_msaa_resolve", 1.0);
}

#[test]
fn webgl2_copytex() {
    run("webgl2_copytex", 1.0);
}

#[test]
fn webgl2_srgb_target() {
    run("webgl2_srgb_target", 1.0);
}

#[test]
fn webgl2_base_level() {
    run("webgl2_base_level", 1.0);
}

#[test]
fn webgl2_r8_rg8() {
    run("webgl2_r8_rg8", 1.0);
}

#[test]
fn webgl2_divisor2() {
    run("webgl2_divisor2", 1.0);
}

#[test]
fn webgl2_float_blit() {
    run("webgl2_float_blit", 1.0);
}

#[test]
fn webgl2_float_blend() {
    run("webgl2_float_blend", 1.0);
}

#[test]
fn a_canvas_with_a_recording_is_one_glsl_layer_at_its_bitmap_size() {
    let recording = case_file("webgl_gradient", "json");
    let body = format!(
        "<canvas id=\"c\" width=\"96\" height=\"64\" data-kg-webgl='{}' style=\"display:block\"></canvas>",
        recording.trim()
    );
    let layers = paintkit::paint_html(&paintkit::page(&body), 200, 100).layers;
    let canvas = layers
        .iter()
        .find(|l| l["id"].as_str().is_some_and(|id| id.starts_with("c")))
        .expect("a glsl layer");
    assert_eq!(canvas["type"], "glsl");
    assert_eq!(canvas["width"].as_f64(), Some(96.0));
    assert!(canvas["ops"].as_array().is_some_and(|ops| !ops.is_empty()));
}

#[test]
fn the_committed_recordings_parse_as_the_scene_format_does() {
    for name in [
        "webgl_gradient",
        "webgl_noise",
        "webgl_texture",
        "webgl_framebuffer",
        "webgl_stencil",
        "webgl2_mrt",
        "webgl2_depth_texture",
        "webgl2_blit_depth_stencil",
        "webgl2_cube_rt",
        "webgl2_mipmap_after_draw",
        "webgl2_msaa_resolve",
        "webgl2_copytex",
        "webgl2_srgb_target",
        "webgl2_base_level",
        "webgl2_r8_rg8",
        "webgl2_divisor2",
        "webgl2_float_blit",
        "webgl2_float_blend",
    ] {
        let content: serde_json::Value = serde_json::from_str(&case_file(name, "json")).unwrap();
        let mut layer = serde_json::json!({ "id": "c", "type": "glsl", "width": 10, "height": 10 });
        layer
            .as_object_mut()
            .unwrap()
            .extend(content.as_object().unwrap().clone());
        let scene = serde_json::json!({
            "width": 10, "height": 10, "fps": 30, "durationInFrames": 1, "layers": [layer],
        });
        kinetograph_core::Scene::from_json(&scene.to_string(), Default::default())
            .unwrap_or_else(|e| panic!("{name}: {e}"));
    }
}
