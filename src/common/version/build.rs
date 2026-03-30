// Copyright 2021 Datafuse Labs
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::env;

#[cfg(feature = "full-build-info")]
fn emit_full_build_info() {
    databend_common_building::setup();
    databend_common_building::setup_commit_authors();
}

#[cfg(not(feature = "full-build-info"))]
fn emit_full_build_info() {}

fn emit_minimal_build_info() {
    println!("cargo:rerun-if-env-changed=DATABEND_RELEASE_VERSION");
    println!("cargo:rerun-if-env-changed=DATABEND_ENTERPRISE_LICENSE_EMBEDDED");
    println!("cargo:rerun-if-env-changed=DATABEND_ENTERPRISE_LICENSE_PUBLIC_KEY");
    println!("cargo:rerun-if-env-changed=DATABEND_TELEMETRY_ENDPOINT");
    println!("cargo:rerun-if-env-changed=DATABEND_TELEMETRY_API_KEY");
    println!("cargo:rerun-if-env-changed=DATABEND_TELEMETRY_SOURCE");
    println!("cargo:rerun-if-env-changed=CARGO_CFG_TARGET_FEATURE");
    println!("cargo:rerun-if-env-changed=PROFILE");
    println!("cargo:rerun-if-env-changed=OPT_LEVEL");

    let version = env::var("DATABEND_RELEASE_VERSION")
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| format!("v{}", env!("CARGO_PKG_VERSION")));
    let profile = env::var("PROFILE").unwrap_or_else(|_| "unknown".to_string());
    let opt_level = env::var("OPT_LEVEL").unwrap_or_else(|_| "unknown".to_string());
    let target_features = env::var("CARGO_CFG_TARGET_FEATURE").unwrap_or_default();
    let telemetry_endpoint = env::var("DATABEND_TELEMETRY_ENDPOINT")
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "https://telemetry.databend.com/v1/report".to_string());
    let telemetry_api_key = env::var("DATABEND_TELEMETRY_API_KEY").unwrap_or_default();
    let telemetry_source = env::var("DATABEND_TELEMETRY_SOURCE").unwrap_or_default();
    let embedded_license = env::var("DATABEND_ENTERPRISE_LICENSE_EMBEDDED").unwrap_or_default();
    let public_key = env::var("DATABEND_ENTERPRISE_LICENSE_PUBLIC_KEY").unwrap_or_default();

    println!("cargo:rustc-env=VERGEN_GIT_SHA=unknown");
    println!("cargo:rustc-env=DATABEND_GIT_SEMVER={version}");
    println!("cargo:rustc-env=DATABEND_COMMIT_AUTHORS=unknown");
    println!("cargo:rustc-env=DATABEND_CREDITS_NAMES=");
    println!("cargo:rustc-env=DATABEND_CREDITS_VERSIONS=");
    println!("cargo:rustc-env=DATABEND_CREDITS_LICENSES=");
    println!("cargo:rustc-env=DATABEND_ENTERPRISE_LICENSE_EMBEDDED={embedded_license}");
    println!("cargo:rustc-env=DATABEND_ENTERPRISE_LICENSE_PUBLIC_KEY={public_key}");
    println!("cargo:rustc-env=DATABEND_CARGO_CFG_TARGET_FEATURE={target_features}");
    println!("cargo:rustc-env=DATABEND_BUILD_PROFILE={profile}");
    println!("cargo:rustc-env=DATABEND_OPT_LEVEL={opt_level}");
    println!("cargo:rustc-env=DATABEND_TELEMETRY_ENDPOINT={telemetry_endpoint}");
    println!("cargo:rustc-env=DATABEND_TELEMETRY_API_KEY={telemetry_api_key}");
    println!("cargo:rustc-env=DATABEND_TELEMETRY_SOURCE={telemetry_source}");
}

fn main() {
    if env::var_os("CARGO_FEATURE_FULL_BUILD_INFO").is_some() {
        emit_full_build_info();
    } else {
        emit_minimal_build_info();
    }
}
