fn main() {
    println!("cargo:rerun-if-changed=vendor");
    println!("cargo:rerun-if-changed=shim");

    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .flag("-w") // vendored code is unmodified upstream C++, warnings not actionable
        // Enables the VNKEY-ORACLE-PATCH reset (vnkeyOracleResetStatics) at the
        // end of vendor/Engine.cpp and its call site in shim/oracle_shim.cpp.
        .define("VNKEY_ORACLE", None)
        .file("vendor/Engine.cpp")
        .file("vendor/Vietnamese.cpp")
        .file("vendor/Macro.cpp")
        .file("vendor/SmartSwitchKey.cpp")
        .file("vendor/ConvertTool.cpp")
        .file("shim/oracle_shim.cpp")
        .include("vendor")
        .compile("openkey_oracle_cpp");
}
