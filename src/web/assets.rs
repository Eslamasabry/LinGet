//! Embedded dashboard assets. Everything is include_str!'d so the binary
//! is self-contained — no runtime file lookups, nothing to install.

pub const INDEX_HTML: &str = include_str!("assets/index.html");
pub const APP_JS: &str = include_str!("assets/app.js");
pub const STYLE_CSS: &str = include_str!("assets/style.css");

pub const THREE_JS: &str = include_str!("assets/vendor/three.module.js");
pub const ORBIT_CONTROLS: &str = include_str!("assets/vendor/addons/controls/OrbitControls.js");
pub const EFFECT_COMPOSER: &str =
    include_str!("assets/vendor/addons/postprocessing/EffectComposer.js");
pub const RENDER_PASS: &str = include_str!("assets/vendor/addons/postprocessing/RenderPass.js");
pub const SHADER_PASS: &str = include_str!("assets/vendor/addons/postprocessing/ShaderPass.js");
pub const MASK_PASS: &str = include_str!("assets/vendor/addons/postprocessing/MaskPass.js");
pub const PASS: &str = include_str!("assets/vendor/addons/postprocessing/Pass.js");
pub const UNREAL_BLOOM: &str =
    include_str!("assets/vendor/addons/postprocessing/UnrealBloomPass.js");
pub const OUTPUT_PASS: &str = include_str!("assets/vendor/addons/postprocessing/OutputPass.js");
pub const COPY_SHADER: &str = include_str!("assets/vendor/addons/shaders/CopyShader.js");
pub const LUMINOSITY_HIGH_PASS: &str =
    include_str!("assets/vendor/addons/shaders/LuminosityHighPassShader.js");
pub const OUTPUT_SHADER: &str = include_str!("assets/vendor/addons/shaders/OutputShader.js");
