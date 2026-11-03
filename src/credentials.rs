//! OS credential store access for context API keys.
//!
//! On macOS, keys are read and written through Apple's `/usr/bin/security`
//! tool rather than the Security framework. Keychain trusts the calling
//! binary's code signature; a locally built `dtrack` is only ad-hoc signed, so
//! each rebuild looks like a new application and asks for the login password
//! again. `/usr/bin/security` has a stable Apple signature, so "Always Allow"
//! is remembered. Elsewhere the `keyring` crate (Secret Service) is used.

use anyhow::Result;

const SERVICE: &str = "dtrack";

#[derive(Debug, thiserror::Error)]
#[error("no API key is stored for context '{0}'")]
pub struct NotFound(pub String);

#[cfg(target_os = "macos")]
mod backend {
    use super::{NotFound, SERVICE};
    use anyhow::{Context, Result};
    use std::{
        io::Write,
        process::{Command, Output, Stdio},
    };

    const SECURITY: &str = "/usr/bin/security";
    /// `errSecItemNotFound`, reported by `security` as its exit status.
    const NOT_FOUND: i32 = 44;

    fn failure(output: &Output, action: &str) -> anyhow::Error {
        anyhow::anyhow!(
            "{action} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )
    }
    fn not_found(output: &Output) -> bool {
        output.status.code() == Some(NOT_FOUND)
    }
    /// Quotes an argument for `security -i`, which splits lines on whitespace
    /// and understands double quotes with backslash escapes.
    fn quote(value: &str) -> String {
        format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
    }
    /// Runs one command through `security -i` so that secrets are passed on
    /// stdin instead of appearing in the process list.
    fn interactive(line: &str) -> Result<Output> {
        let mut child = Command::new(SECURITY)
            .arg("-i")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("running /usr/bin/security")?;
        let mut stdin = child
            .stdin
            .take()
            .context("opening /usr/bin/security stdin")?;
        stdin.write_all(line.as_bytes())?;
        stdin.write_all(b"\n")?;
        drop(stdin);
        Ok(child.wait_with_output()?)
    }

    pub fn get(name: &str) -> Result<String> {
        let output = Command::new(SECURITY)
            .args(["find-generic-password", "-s", SERVICE, "-a", name, "-w"])
            .stdin(Stdio::null())
            .output()
            .context("running /usr/bin/security")?;
        if not_found(&output) {
            return Err(NotFound(name.to_owned()).into());
        }
        if !output.status.success() {
            return Err(failure(&output, "reading the Keychain item"));
        }
        let key = String::from_utf8(output.stdout).context("stored API key is not UTF-8")?;
        Ok(key.trim_end_matches('\n').to_owned())
    }
    pub fn set(name: &str, key: &str) -> Result<()> {
        // Replace rather than update: an item created by an earlier dtrack
        // build keeps an access list that only trusts that build.
        delete(name)?;
        let hex: String = key.bytes().map(|byte| format!("{byte:02x}")).collect();
        let output = interactive(&format!(
            "add-generic-password -s {} -a {} -X {hex} -T {SECURITY}",
            quote(SERVICE),
            quote(name)
        ))?;
        if !output.status.success() {
            return Err(failure(&output, "storing the Keychain item"));
        }
        Ok(())
    }
    pub fn delete(name: &str) -> Result<()> {
        let output = Command::new(SECURITY)
            .args(["delete-generic-password", "-s", SERVICE, "-a", name])
            .stdin(Stdio::null())
            .output()
            .context("running /usr/bin/security")?;
        if output.status.success() || not_found(&output) {
            Ok(())
        } else {
            Err(failure(&output, "deleting the Keychain item"))
        }
    }

    #[cfg(test)]
    mod tests {
        #[test]
        fn quoting_escapes_quotes_and_backslashes() {
            assert_eq!(super::quote(r#"a "b"\c"#), r#""a \"b\"\\c""#);
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod backend {
    use super::{NotFound, SERVICE};
    use anyhow::Result;

    fn entry(name: &str) -> Result<keyring::Entry> {
        Ok(keyring::Entry::new(SERVICE, name)?)
    }
    pub fn get(name: &str) -> Result<String> {
        match entry(name)?.get_password() {
            Err(keyring::Error::NoEntry) => Err(NotFound(name.to_owned()).into()),
            result => Ok(result?),
        }
    }
    pub fn set(name: &str, key: &str) -> Result<()> {
        Ok(entry(name)?.set_password(key)?)
    }
    pub fn delete(name: &str) -> Result<()> {
        match entry(name)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(error.into()),
        }
    }
}

/// Reads the stored API key of a context.
pub fn get(name: &str) -> Result<String> {
    backend::get(name)
}
/// Stores the API key of a context, replacing any existing one.
pub fn set(name: &str, key: &str) -> Result<()> {
    backend::set(name, key)
}
/// Deletes the stored API key of a context. A missing key is not an error.
pub fn delete(name: &str) -> Result<()> {
    backend::delete(name)
}
