use std::{
    path::PathBuf,
    process::{Command, Stdio},
};

use super::prelude::*;

/// tvOS support (POC). Mirrors the visionOS strategy: Skia's GN build has no tvOS
/// notion, so it is built as `target_os = "ios"` with the tvOS SDK sysroot and a
/// tvOS clang target triple.
pub struct TvOs;

// tvOS 12.0 is the oldest SDK that ships the APIs Skia's iOS code paths use
// (matches the iOS 12 minimum the `ios` module uses).
const MIN_TVOS_VERSION: &str = "12.0";

impl PlatformDetails for TvOs {
    fn uses_freetype(&self) -> bool {
        false
    }

    fn gn_args(&self, config: &BuildConfiguration, builder: &mut GnArgsBuilder) {
        let platform = TvOsPlatform::new(&config.target);

        builder.target_os_and_default_cpu("ios");

        if platform.is_simulator() {
            builder.arg("ios_use_simulator", yes());
        }

        // Without this, Skia auto-resolves `xcode_sysroot` to the iphoneos/iphonesimulator SDK.
        builder.arg("xcode_sysroot", quote(platform.sdk_path().to_str().unwrap()));

        // The triple carries platform + minimum version, so no `-m*-version-min` is needed.
        builder.target(platform.clang_target());
    }

    fn bindgen_args(&self, target: &Target, builder: &mut BindgenArgsBuilder) {
        let platform = TvOsPlatform::new(target);

        builder.arg("-isysroot");
        builder.arg(platform.sdk_path().to_str().unwrap().to_string());

        builder.override_target(&platform.clang_target());
    }

    fn link_libraries(&self, features: &Features) -> Vec<String> {
        let mut libs = vec![
            "c++",
            "framework=CoreFoundation",
            "framework=CoreGraphics",
            "framework=CoreText",
            "framework=ImageIO",
            "framework=MobileCoreServices",
            "framework=UIKit",
        ];

        if features[feature::METAL] {
            libs.push("framework=Metal");
        }

        libs.iter().map(|s| s.to_string()).collect()
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct TvOsPlatform {
    arch: String,
    simulator: bool,
}

impl TvOsPlatform {
    fn new(target: &Target) -> Self {
        // `aarch64-apple-tvos-sim` (abi `sim`) and `x86_64-apple-tvos` are simulator targets;
        // `aarch64-apple-tvos` is the device target.
        let (arch, abi) = target.arch_abi();
        let simulator = matches!((arch, abi), (_, Some("sim")) | ("x86_64", _));
        Self {
            arch: clang::target_arch(arch).to_string(),
            simulator,
        }
    }

    fn is_simulator(&self) -> bool {
        self.simulator
    }

    fn clang_target(&self) -> String {
        if self.is_simulator() {
            format!("{}-apple-tvos{MIN_TVOS_VERSION}-simulator", self.arch)
        } else {
            format!("{}-apple-tvos{MIN_TVOS_VERSION}", self.arch)
        }
    }

    fn sdk_name(&self) -> &'static str {
        if self.is_simulator() {
            "appletvsimulator"
        } else {
            "appletvos"
        }
    }

    /// Resolve the tvOS SDK path by starting `xcrun`.
    fn sdk_path(&self) -> PathBuf {
        let sdk_path = Command::new("xcrun")
            .arg("--show-sdk-path")
            .arg("--sdk")
            .arg(self.sdk_name())
            .stderr(Stdio::inherit())
            .output()
            .expect("Failed to invoke xcrun")
            .stdout;

        let string = String::from_utf8(sdk_path).expect("failed to resolve tvOS SDK path");
        PathBuf::from(string.trim())
    }
}
