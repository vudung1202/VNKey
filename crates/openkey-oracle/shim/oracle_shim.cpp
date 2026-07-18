// oracle_shim.cpp
//
// extern "C" bridge between Rust (crates/openkey-oracle/src/lib.rs) and the
// vendored OpenKey C++ engine (crates/openkey-oracle/vendor/). This crate is
// a dev-only test oracle: it exists so differential tests can compare
// vnkey-engine's pure-Rust port against the original C++ behavior.
//
// OpenKey's engine expects its "host app" (the macOS/Windows/Linux OpenKey
// application) to define the ~20 option globals declared `extern` in
// Engine.h, and to call vKeyInit() once before use. This shim IS that host
// app, for test purposes only.

#include <cstdint>
#include <cstring>
#include <vector>

#include "../vendor/Engine.h"

// -- option globals declared `extern` in Engine.h, defined by the host app
// in upstream OpenKey. This oracle is the host app for test purposes. --
int vLanguage = 1;
int vInputType = 0;
int vFreeMark = 0;
int vCodeTable = 0;
int vSwitchKeyStatus = 0;
int vCheckSpelling = 0;
int vUseModernOrthography = 0;
int vQuickTelex = 0;
int vRestoreIfWrongSpelling = 0;
int vFixRecommendBrowser = 0;
int vUseMacro = 0;
int vUseMacroInEnglishMode = 0;
int vAutoCapsMacro = 0;
int vUseSmartSwitchKey = 0;
int vUpperCaseFirstChar = 0;
int vTempOffSpelling = 0;
int vAllowConsonantZFWJ = 0;
int vQuickStartConsonant = 0;
int vQuickEndConsonant = 0;
int vRememberCode = 0;
int vOtherLanguage = 0;
int vTempOffOpenKey = 0;

// vKeyHookState* returned by vKeyInit(); ok_get_state reads through this
// pointer rather than declaring its own extern for the (header-undeclared)
// `HookState` global that Engine.cpp defines.
static vKeyHookState* g_state = nullptr;

#ifdef VNKEY_ORACLE
// VNKEY-ORACLE-PATCH: defined at the end of vendor/Engine.cpp under the same
// guard. Resets every Engine.cpp file-scope static + HookState so each
// ok_reset() hands back a pristine engine (vKeyInit alone leaks state between
// sessions). Declared here because those statics are private to Engine.cpp.
void vnkeyOracleResetStatics();
// VNKEY-ORACLE-PATCH: defined at the end of vendor/SmartSwitchKey.cpp under the
// same guard. Resets the smart-switch 1-entry read cache (initSmartSwitchKey
// clears the map only), so cache state does not leak across locked sessions.
void vnkeyOracleResetSmartSwitchCache();
#endif

extern "C" {

typedef struct {
    int32_t language, input_type, free_mark, code_table, switch_key_status,
            check_spelling, use_modern_orthography, quick_telex,
            restore_if_wrong_spelling, fix_recommend_browser, use_macro,
            use_macro_in_english_mode, auto_caps_macro, use_smart_switch_key,
            upper_case_first_char, temp_off_spelling, allow_consonant_zfwj,
            quick_start_consonant, quick_end_consonant, remember_code,
            other_language, temp_off_openkey;
} OkOptions;

typedef struct {
    uint8_t code, backspace_count, new_char_count, ext_code;
    uint32_t char_data[32];
    uint32_t macro_key_len;  uint32_t macro_key[256];
    uint32_t macro_data_len; uint32_t macro_data[1024];
} OkState;

static void applyOptions(const OkOptions* opts) {
    vLanguage = opts->language;
    vInputType = opts->input_type;
    vFreeMark = opts->free_mark;
    vCodeTable = opts->code_table;
    vSwitchKeyStatus = opts->switch_key_status;
    vCheckSpelling = opts->check_spelling;
    vUseModernOrthography = opts->use_modern_orthography;
    vQuickTelex = opts->quick_telex;
    vRestoreIfWrongSpelling = opts->restore_if_wrong_spelling;
    vFixRecommendBrowser = opts->fix_recommend_browser;
    vUseMacro = opts->use_macro;
    vUseMacroInEnglishMode = opts->use_macro_in_english_mode;
    vAutoCapsMacro = opts->auto_caps_macro;
    vUseSmartSwitchKey = opts->use_smart_switch_key;
    vUpperCaseFirstChar = opts->upper_case_first_char;
    vTempOffSpelling = opts->temp_off_spelling;
    vAllowConsonantZFWJ = opts->allow_consonant_zfwj;
    vQuickStartConsonant = opts->quick_start_consonant;
    vQuickEndConsonant = opts->quick_end_consonant;
    vRememberCode = opts->remember_code;
    vOtherLanguage = opts->other_language;
    vTempOffOpenKey = opts->temp_off_openkey;
}

void ok_reset(const OkOptions* opts) {
    applyOptions(opts);
    g_state = static_cast<vKeyHookState*>(vKeyInit());
#ifdef VNKEY_ORACLE
    vnkeyOracleResetStatics(); // full static reset; must run after vKeyInit()
    vnkeyOracleResetSmartSwitchCache(); // clear the smart-switch read cache
#endif
    initMacroMap(nullptr, 0);
    initSmartSwitchKey(nullptr, 0);
}

void ok_set_options(const OkOptions* opts) {
    applyOptions(opts);
}

void ok_handle_event(uint8_t event, uint8_t state, uint16_t keycode, uint8_t caps, uint8_t other_control) {
    vKeyHandleEvent(static_cast<vKeyEvent>(event), static_cast<vKeyEventState>(state), keycode, caps, other_control != 0);
}

void ok_english_mode(uint8_t state, uint16_t keycode, uint8_t caps, uint8_t other_control) {
    vEnglishMode(static_cast<vKeyEventState>(state), keycode, caps != 0, other_control != 0);
}

void ok_start_new_session(void) {
    startNewSession();
}

void ok_temp_off_spell_checking(void) {
    vTempOffSpellChecking();
}

void ok_set_check_spelling(void) {
    vSetCheckSpelling();
}

void ok_temp_off_engine(uint8_t off) {
    vTempOffEngine(off != 0);
}

void ok_get_state(OkState* out) {
    std::memset(out, 0, sizeof(OkState));
    if (g_state == nullptr) {
        return; // ok_reset (which sets g_state) was never called
    }

    out->code = static_cast<uint8_t>(g_state->code);
    out->backspace_count = g_state->backspaceCount;
    out->new_char_count = g_state->newCharCount;
    out->ext_code = g_state->extCode;
    for (int i = 0; i < 32; i++) {
        out->char_data[i] = g_state->charData[i];
    }

    uint32_t keyLen = static_cast<uint32_t>(g_state->macroKey.size());
    if (keyLen > 256) keyLen = 256;
    out->macro_key_len = keyLen;
    for (uint32_t i = 0; i < keyLen; i++) {
        out->macro_key[i] = g_state->macroKey[i];
    }

    uint32_t dataLen = static_cast<uint32_t>(g_state->macroData.size());
    if (dataLen > 1024) dataLen = 1024;
    out->macro_data_len = dataLen;
    for (uint32_t i = 0; i < dataLen; i++) {
        out->macro_data[i] = g_state->macroData[i];
    }
}

uint32_t ok_get_character_code(int32_t code_table, uint32_t cell) {
    vCodeTable = code_table;
    return getCharacterCode(cell);
}

uint32_t ok_key_code_to_character(uint32_t key_code) {
    return static_cast<uint32_t>(keyCodeToCharacter(key_code));
}

void ok_set_macro_blob(const uint8_t* data, uint32_t len) {
    initMacroMap(reinterpret_cast<const Byte*>(data), static_cast<int>(len));
}

uint32_t ok_get_macro_blob(uint8_t* out, uint32_t cap) {
    std::vector<Byte> buf;
    getMacroSaveData(buf);
    uint32_t total = static_cast<uint32_t>(buf.size());
    uint32_t n = total < cap ? total : cap;
    if (out != nullptr && n > 0) {
        std::memcpy(out, buf.data(), n);
    }
    return total;
}

// -- Macro: direct addMacro / findMacro / onTableCodeChange (differential) --

void ok_add_macro(const char* text, uint32_t tlen, const char* content, uint32_t clen) {
    addMacro(std::string(text, tlen), std::string(content, clen));
}

void ok_macro_on_table_change(int32_t code_table) {
    vCodeTable = code_table;
    onTableCodeChange();
}

// Runs findMacro(key, out) under vCodeTable=code_table, vAutoCapsMacro=auto_caps.
// Returns 0xFFFFFFFF when no macro matched (distinct from a matched macro with
// empty content, which returns 0); otherwise the full content length (the
// caller truncates to `cap`).
uint32_t ok_find_macro(const uint32_t* key, uint32_t key_len, int32_t code_table,
                       int32_t auto_caps, uint32_t* out, uint32_t cap) {
    vCodeTable = code_table;
    vAutoCapsMacro = auto_caps;
    std::vector<Uint32> k(key, key + key_len);
    std::vector<Uint32> data;
    if (!findMacro(k, data)) {
        return 0xFFFFFFFFu;
    }
    uint32_t total = static_cast<uint32_t>(data.size());
    uint32_t n = total < cap ? total : cap;
    for (uint32_t i = 0; i < n; i++) {
        out[i] = data[i];
    }
    return total;
}

// -- SmartSwitchKey --

void ok_smart_set_blob(const uint8_t* data, uint32_t len) {
    initSmartSwitchKey(reinterpret_cast<const Byte*>(data), static_cast<int>(len));
}

uint32_t ok_smart_get_blob(uint8_t* out, uint32_t cap) {
    std::vector<Byte> buf;
    getSmartSwitchKeySaveData(buf);
    uint32_t total = static_cast<uint32_t>(buf.size());
    uint32_t n = total < cap ? total : cap;
    if (out != nullptr && n > 0) {
        std::memcpy(out, buf.data(), n);
    }
    return total;
}

int32_t ok_smart_get(const char* bundle, uint32_t len, int32_t current) {
    return getAppInputMethodStatus(std::string(bundle, len), current);
}

void ok_smart_set(const char* bundle, uint32_t len, int32_t language) {
    setAppInputMethodStatus(std::string(bundle, len), language);
}

// -- ConvertTool --

typedef struct {
    int32_t from_code, to_code, remove_mark, to_all_caps, to_all_non_caps,
            to_caps_first_letter, to_caps_each_word;
} OkConvertOpts;

uint32_t ok_convert_util(const char* src, uint32_t len, const OkConvertOpts* o,
                         uint8_t* out, uint32_t cap) {
    convertToolFromCode = static_cast<Uint8>(o->from_code);
    convertToolToCode = static_cast<Uint8>(o->to_code);
    convertToolRemoveMark = o->remove_mark != 0;
    convertToolToAllCaps = o->to_all_caps != 0;
    convertToolToAllNonCaps = o->to_all_non_caps != 0;
    convertToolToCapsFirstLetter = o->to_caps_first_letter != 0;
    convertToolToCapsEachWord = o->to_caps_each_word != 0;
    std::string result = convertUtil(std::string(src, len));
    uint32_t total = static_cast<uint32_t>(result.size());
    uint32_t n = total < cap ? total : cap;
    if (out != nullptr && n > 0) {
        std::memcpy(out, result.data(), n);
    }
    return total;
}

} // extern "C"
