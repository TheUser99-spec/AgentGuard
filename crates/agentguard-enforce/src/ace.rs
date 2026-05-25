use agentguard_core::{GuardError, GuardResult};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProtectionHealth {
    pub exists: bool,
    pub content_deny: bool,
    pub metadata_deny: bool,
    pub mic_high_no_write_up: bool,
}

impl ProtectionHealth {
    pub fn healthy(&self) -> bool {
        self.exists && self.content_deny && self.metadata_deny && self.mic_high_no_write_up
    }
}

pub fn apply_deny_ace(path: &Path) -> GuardResult<()> {
    if !path.exists() {
        return Ok(());
    }

    #[cfg(windows)]
    return win_api::apply_deny_ace_impl(path);

    #[cfg(not(windows))]
    return dev::mark_denied(path);
}

pub fn remove_deny_ace(path: &Path) -> GuardResult<()> {
    #[cfg(windows)]
    return win_api::remove_deny_ace_impl(path);

    #[cfg(not(windows))]
    return dev::unmark_denied(path);
}

pub fn verify_ace(path: &Path) -> GuardResult<ProtectionHealth> {
    #[cfg(windows)]
    return win_api::verify_ace_impl(path);

    #[cfg(not(windows))]
    return Ok(dev::health(path));
}

#[cfg(windows)]
mod win_api {
    use super::*;
    use std::ffi::c_void;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::{LocalFree, GENERIC_ALL};
    use windows_sys::Win32::Security::Authorization::{
        ConvertStringSidToSidW, GetNamedSecurityInfoW, SetEntriesInAclW, SetNamedSecurityInfoW,
        DENY_ACCESS, EXPLICIT_ACCESS_W, NO_MULTIPLE_TRUSTEE, SE_FILE_OBJECT, TRUSTEE_IS_SID,
        TRUSTEE_IS_WELL_KNOWN_GROUP, TRUSTEE_W,
    };
    use windows_sys::Win32::Security::{
        AddAce, AddMandatoryAce, EqualSid, GetAce, GetLengthSid, InitializeAcl, ACCESS_DENIED_ACE,
        ACE_HEADER, ACL, ACL_REVISION, DACL_SECURITY_INFORMATION, LABEL_SECURITY_INFORMATION,
        NO_INHERITANCE, PROTECTED_DACL_SECURITY_INFORMATION, PROTECTED_SACL_SECURITY_INFORMATION,
        PSECURITY_DESCRIPTOR, PSID, SYSTEM_MANDATORY_LABEL_ACE,
        UNPROTECTED_DACL_SECURITY_INFORMATION, UNPROTECTED_SACL_SECURITY_INFORMATION,
    };
    use windows_sys::Win32::Storage::FileSystem::{DELETE, WRITE_DAC, WRITE_OWNER};

    const ACCESS_DENIED_ACE_TYPE: u8 = 0x01;
    const SYSTEM_MANDATORY_LABEL_ACE_TYPE: u8 = 0x11;
    const SYSTEM_MANDATORY_LABEL_NO_WRITE_UP: u32 = 0x01;
    const MAXDWORD: u32 = u32::MAX;

    fn to_wide_str(s: &str) -> Vec<u16> {
        OsStr::new(s)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    fn to_wide(path: &Path) -> Vec<u16> {
        to_wide_str(&path.to_string_lossy())
    }

    #[derive(Debug)]
    struct LocalPtr(*mut c_void);

    impl LocalPtr {
        fn new(ptr: *mut c_void) -> Self {
            Self(ptr)
        }

        fn as_psid(&self) -> PSID {
            self.0 as PSID
        }
    }

    impl Drop for LocalPtr {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    LocalFree(self.0);
                }
            }
        }
    }

    pub fn apply_deny_ace_impl(path: &Path) -> GuardResult<()> {
        let wide = to_wide(path);

        let mut p_dacl: *mut ACL = std::ptr::null_mut();
        let mut p_sd: PSECURITY_DESCRIPTOR = std::ptr::null_mut();

        let r = unsafe {
            GetNamedSecurityInfoW(
                wide.as_ptr(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut p_dacl,
                std::ptr::null_mut(),
                &mut p_sd,
            )
        };
        let _sd = LocalPtr::new(p_sd);
        if r != 0 {
            return Err(GuardError::EnforcementFailed {
                path: path.display().to_string(),
                reason: format!("GetNamedSecurityInfoW: {r}"),
            });
        }

        let everyone_sid = build_everyone_sid()?;
        let high_sid = build_high_integrity_sid()?;
        let cleaned_dacl = acl_without_agentguard_deny(p_dacl, everyone_sid.as_psid(), 0)?;

        let mut entries = [
            EXPLICIT_ACCESS_W {
                grfAccessPermissions: GENERIC_ALL,
                grfAccessMode: DENY_ACCESS,
                grfInheritance: NO_INHERITANCE,
                Trustee: trustee_for_sid(everyone_sid.as_psid()),
            },
            EXPLICIT_ACCESS_W {
                grfAccessPermissions: metadata_mask(),
                grfAccessMode: DENY_ACCESS,
                grfInheritance: NO_INHERITANCE,
                Trustee: trustee_for_sid(everyone_sid.as_psid()),
            },
        ];

        let mut new_dacl: *mut ACL = std::ptr::null_mut();
        let r = unsafe {
            SetEntriesInAclW(
                entries.len() as u32,
                entries.as_mut_ptr(),
                cleaned_dacl.as_ptr() as *mut ACL,
                &mut new_dacl,
            )
        };
        let new_dacl = LocalPtr::new(new_dacl as *mut c_void);
        if r != 0 {
            return Err(GuardError::EnforcementFailed {
                path: path.display().to_string(),
                reason: format!("SetEntriesInAclW: {r}"),
            });
        }

        apply_mic_label(path, &wide, high_sid.as_psid())?;

        let r = unsafe {
            SetNamedSecurityInfoW(
                wide.as_ptr(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                new_dacl.0 as *mut ACL,
                std::ptr::null(),
            )
        };
        if r != 0 {
            let _ = remove_mic_label(path, &wide, high_sid.as_psid());
            return Err(GuardError::EnforcementFailed {
                path: path.display().to_string(),
                reason: format!("SetNamedSecurityInfoW: {r}"),
            });
        }

        Ok(())
    }

    pub fn remove_deny_ace_impl(path: &Path) -> GuardResult<()> {
        let wide = to_wide(path);

        let everyone_sid = build_everyone_sid()?;
        let high_sid = build_high_integrity_sid()?;

        let mut p_dacl: *mut ACL = std::ptr::null_mut();
        let mut p_sd: PSECURITY_DESCRIPTOR = std::ptr::null_mut();

        let r = unsafe {
            GetNamedSecurityInfoW(
                wide.as_ptr(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut p_dacl,
                std::ptr::null_mut(),
                &mut p_sd,
            )
        };
        let _sd = LocalPtr::new(p_sd);
        if r != 0 {
            return Err(GuardError::EnforcementFailed {
                path: path.display().to_string(),
                reason: format!("GetNamedSecurityInfoW remove DACL: {r}"),
            });
        }

        let cleaned_dacl = acl_without_agentguard_deny(p_dacl, everyone_sid.as_psid(), 0)?;

        let r = unsafe {
            SetNamedSecurityInfoW(
                wide.as_ptr(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION | UNPROTECTED_DACL_SECURITY_INFORMATION,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                cleaned_dacl.as_ptr() as *mut ACL,
                std::ptr::null(),
            )
        };
        if r != 0 {
            return Err(GuardError::EnforcementFailed {
                path: path.display().to_string(),
                reason: format!("SetNamedSecurityInfoW remove: {r}"),
            });
        }

        remove_mic_label(path, &wide, high_sid.as_psid())?;

        Ok(())
    }

    pub fn verify_ace_impl(path: &Path) -> GuardResult<ProtectionHealth> {
        if !path.exists() {
            return Ok(ProtectionHealth {
                exists: false,
                ..ProtectionHealth::default()
            });
        }

        let wide = to_wide(path);
        let everyone_sid = build_everyone_sid()?;
        let high_sid = build_high_integrity_sid()?;

        let mut p_dacl: *mut ACL = std::ptr::null_mut();
        let mut p_sacl: *mut ACL = std::ptr::null_mut();
        let mut p_sd: PSECURITY_DESCRIPTOR = std::ptr::null_mut();

        let r = unsafe {
            GetNamedSecurityInfoW(
                wide.as_ptr(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION | LABEL_SECURITY_INFORMATION,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut p_dacl,
                &mut p_sacl,
                &mut p_sd,
            )
        };
        let _sd = LocalPtr::new(p_sd);
        if r != 0 {
            return Err(GuardError::EnforcementFailed {
                path: path.display().to_string(),
                reason: format!("GetNamedSecurityInfoW verify: {r}"),
            });
        }

        Ok(ProtectionHealth {
            exists: true,
            content_deny: has_content_deny(p_dacl, everyone_sid.as_psid())?,
            metadata_deny: has_metadata_deny(p_dacl, everyone_sid.as_psid())?,
            mic_high_no_write_up: has_high_no_write_up_label(p_sacl, high_sid.as_psid())?,
        })
    }

    fn trustee_for_sid(sid: PSID) -> TRUSTEE_W {
        TRUSTEE_W {
            pMultipleTrustee: std::ptr::null_mut(),
            MultipleTrusteeOperation: NO_MULTIPLE_TRUSTEE,
            TrusteeForm: TRUSTEE_IS_SID,
            TrusteeType: TRUSTEE_IS_WELL_KNOWN_GROUP,
            ptstrName: sid as *mut u16,
        }
    }

    fn build_everyone_sid() -> GuardResult<LocalPtr> {
        build_sid("S-1-1-0")
    }

    fn build_high_integrity_sid() -> GuardResult<LocalPtr> {
        build_sid("S-1-16-12288")
    }

    fn build_sid(sddl: &str) -> GuardResult<LocalPtr> {
        let sid_str = to_wide_str(sddl);
        let mut sid: PSID = std::ptr::null_mut();
        let ok = unsafe { ConvertStringSidToSidW(sid_str.as_ptr(), &mut sid) };
        if ok == 0 {
            return Err(GuardError::EnforcementFailed {
                path: sddl.into(),
                reason: format!("ConvertStringSidToSidW failed for {sddl}"),
            });
        }
        Ok(LocalPtr::new(sid))
    }

    fn metadata_mask() -> u32 {
        WRITE_DAC | WRITE_OWNER | DELETE
    }

    fn acl_without_agentguard_deny(
        acl: *const ACL,
        everyone_sid: PSID,
        extra_bytes: u32,
    ) -> GuardResult<Vec<u8>> {
        copy_acl_without(acl, extra_bytes, |ace| {
            is_agentguard_deny_ace(ace, everyone_sid).unwrap_or(false)
        })
    }

    fn copy_acl_without<F>(
        acl: *const ACL,
        extra_bytes: u32,
        mut should_remove: F,
    ) -> GuardResult<Vec<u8>>
    where
        F: FnMut(*const ACE_HEADER) -> bool,
    {
        let base_size = if acl.is_null() {
            std::mem::size_of::<ACL>() as u32
        } else {
            unsafe { (*acl).AclSize as u32 }
        };
        let new_size = base_size.saturating_add(extra_bytes);
        let mut buffer = vec![0u8; new_size as usize];
        let new_acl = buffer.as_mut_ptr() as *mut ACL;

        let ok = unsafe { InitializeAcl(new_acl, new_size, ACL_REVISION) };
        if ok == 0 {
            return Err(GuardError::EnforcementFailed {
                path: "ACL".into(),
                reason: "InitializeAcl failed".into(),
            });
        }

        if acl.is_null() {
            return Ok(buffer);
        }

        let ace_count = unsafe { (*acl).AceCount as u32 };
        for i in 0..ace_count {
            let mut ace: *mut c_void = std::ptr::null_mut();
            let ok = unsafe { GetAce(acl, i, &mut ace) };
            if ok == 0 {
                return Err(GuardError::EnforcementFailed {
                    path: "ACL".into(),
                    reason: format!("GetAce failed at index {i}"),
                });
            }

            let header = ace as *const ACE_HEADER;
            if !should_remove(header) {
                let ace_size = unsafe { (*header).AceSize as u32 };
                let ok = unsafe { AddAce(new_acl, ACL_REVISION, MAXDWORD, ace, ace_size) };
                if ok == 0 {
                    return Err(GuardError::EnforcementFailed {
                        path: "ACL".into(),
                        reason: format!("AddAce failed at index {i}"),
                    });
                }
            }
        }

        Ok(buffer)
    }

    fn apply_mic_label(path: &Path, wide: &[u16], high_sid: PSID) -> GuardResult<()> {
        let mut p_sacl: *mut ACL = std::ptr::null_mut();
        let mut p_sd: PSECURITY_DESCRIPTOR = std::ptr::null_mut();

        let r = unsafe {
            GetNamedSecurityInfoW(
                wide.as_ptr(),
                SE_FILE_OBJECT,
                LABEL_SECURITY_INFORMATION,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut p_sacl,
                &mut p_sd,
            )
        };
        let _sd = LocalPtr::new(p_sd);
        if r != 0 {
            return Err(GuardError::EnforcementFailed {
                path: path.display().to_string(),
                reason: format!("GetNamedSecurityInfoW MIC: {r}"),
            });
        }

        let extra = mandatory_ace_size(high_sid);
        let mut sacl = copy_acl_without(p_sacl, extra, |ace| {
            is_agentguard_mic_ace(ace, high_sid).unwrap_or(false)
        })?;
        let new_sacl = sacl.as_mut_ptr() as *mut ACL;

        let ok = unsafe {
            AddMandatoryAce(
                new_sacl,
                ACL_REVISION,
                NO_INHERITANCE,
                SYSTEM_MANDATORY_LABEL_NO_WRITE_UP,
                high_sid,
            )
        };
        if ok == 0 {
            return Err(GuardError::EnforcementFailed {
                path: path.display().to_string(),
                reason: "AddMandatoryAce failed".into(),
            });
        }

        let r = unsafe {
            SetNamedSecurityInfoW(
                wide.as_ptr(),
                SE_FILE_OBJECT,
                LABEL_SECURITY_INFORMATION | PROTECTED_SACL_SECURITY_INFORMATION,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                new_sacl,
            )
        };
        if r != 0 {
            return Err(GuardError::EnforcementFailed {
                path: path.display().to_string(),
                reason: format!("SetNamedSecurityInfoW MIC: {r}"),
            });
        }

        Ok(())
    }

    fn remove_mic_label(path: &Path, wide: &[u16], high_sid: PSID) -> GuardResult<()> {
        let mut p_sacl: *mut ACL = std::ptr::null_mut();
        let mut p_sd: PSECURITY_DESCRIPTOR = std::ptr::null_mut();

        let r = unsafe {
            GetNamedSecurityInfoW(
                wide.as_ptr(),
                SE_FILE_OBJECT,
                LABEL_SECURITY_INFORMATION,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut p_sacl,
                &mut p_sd,
            )
        };
        let _sd = LocalPtr::new(p_sd);
        if r != 0 {
            return Err(GuardError::EnforcementFailed {
                path: path.display().to_string(),
                reason: format!("GetNamedSecurityInfoW remove MIC: {r}"),
            });
        }

        let mut sacl = copy_acl_without(p_sacl, 0, |ace| {
            is_agentguard_mic_ace(ace, high_sid).unwrap_or(false)
        })?;
        let new_sacl = sacl.as_mut_ptr() as *mut ACL;

        let r = unsafe {
            SetNamedSecurityInfoW(
                wide.as_ptr(),
                SE_FILE_OBJECT,
                LABEL_SECURITY_INFORMATION | UNPROTECTED_SACL_SECURITY_INFORMATION,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                new_sacl,
            )
        };
        if r != 0 {
            return Err(GuardError::EnforcementFailed {
                path: path.display().to_string(),
                reason: format!("SetNamedSecurityInfoW remove MIC: {r}"),
            });
        }

        Ok(())
    }

    fn mandatory_ace_size(sid: PSID) -> u32 {
        (std::mem::size_of::<SYSTEM_MANDATORY_LABEL_ACE>() - std::mem::size_of::<u32>()) as u32
            + unsafe { GetLengthSid(sid) }
    }

    fn has_content_deny(acl: *const ACL, everyone_sid: PSID) -> GuardResult<bool> {
        find_ace(acl, |ace| {
            let Some(mask) = access_denied_mask_for_sid(ace, everyone_sid)? else {
                return Ok(false);
            };
            Ok(mask == GENERIC_ALL)
        })
    }

    fn has_metadata_deny(acl: *const ACL, everyone_sid: PSID) -> GuardResult<bool> {
        find_ace(acl, |ace| {
            let Some(mask) = access_denied_mask_for_sid(ace, everyone_sid)? else {
                return Ok(false);
            };
            Ok((mask & metadata_mask()) == metadata_mask())
        })
    }

    fn has_high_no_write_up_label(acl: *const ACL, high_sid: PSID) -> GuardResult<bool> {
        find_ace(acl, |ace| is_agentguard_mic_ace(ace, high_sid))
    }

    fn find_ace<F>(acl: *const ACL, mut predicate: F) -> GuardResult<bool>
    where
        F: FnMut(*const ACE_HEADER) -> GuardResult<bool>,
    {
        if acl.is_null() {
            return Ok(false);
        }

        let ace_count = unsafe { (*acl).AceCount as u32 };
        for i in 0..ace_count {
            let mut ace: *mut c_void = std::ptr::null_mut();
            let ok = unsafe { GetAce(acl, i, &mut ace) };
            if ok == 0 {
                return Err(GuardError::EnforcementFailed {
                    path: "ACL".into(),
                    reason: format!("GetAce failed at index {i}"),
                });
            }
            if predicate(ace as *const ACE_HEADER)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn is_agentguard_deny_ace(ace: *const ACE_HEADER, everyone_sid: PSID) -> GuardResult<bool> {
        let Some(mask) = access_denied_mask_for_sid(ace, everyone_sid)? else {
            return Ok(false);
        };
        Ok(mask == GENERIC_ALL || (mask & metadata_mask()) == metadata_mask())
    }

    fn access_denied_mask_for_sid(ace: *const ACE_HEADER, sid: PSID) -> GuardResult<Option<u32>> {
        if ace.is_null() || unsafe { (*ace).AceType } != ACCESS_DENIED_ACE_TYPE {
            return Ok(None);
        }

        let deny = unsafe { &*(ace as *const ACCESS_DENIED_ACE) };
        let ace_sid = (&deny.SidStart as *const u32) as PSID;
        let ok = unsafe { EqualSid(ace_sid, sid) };
        if ok == 0 {
            return Ok(None);
        }
        Ok(Some(deny.Mask))
    }

    fn is_agentguard_mic_ace(ace: *const ACE_HEADER, high_sid: PSID) -> GuardResult<bool> {
        if ace.is_null() || unsafe { (*ace).AceType } != SYSTEM_MANDATORY_LABEL_ACE_TYPE {
            return Ok(false);
        }

        let label = unsafe { &*(ace as *const SYSTEM_MANDATORY_LABEL_ACE) };
        if (label.Mask & SYSTEM_MANDATORY_LABEL_NO_WRITE_UP) == 0 {
            return Ok(false);
        }

        let label_sid = (&label.SidStart as *const u32) as PSID;
        let ok = unsafe { EqualSid(label_sid, high_sid) };
        Ok(ok != 0)
    }
}

#[cfg(not(windows))]
mod dev {
    use super::*;

    fn marker_path(path: &Path) -> std::path::PathBuf {
        let name = format!(
            ".agentguard-deny-{}",
            path.file_name().unwrap_or_default().to_string_lossy()
        );
        path.parent().unwrap_or(path).join(name)
    }

    pub fn mark_denied(path: &Path) -> GuardResult<()> {
        let marker = marker_path(path);
        std::fs::write(&marker, path.to_string_lossy().as_bytes()).map_err(|e| {
            GuardError::EnforcementFailed {
                path: path.display().to_string(),
                reason: e.to_string(),
            }
        })
    }

    pub fn unmark_denied(path: &Path) -> GuardResult<()> {
        let marker = marker_path(path);
        if marker.exists() {
            std::fs::remove_file(&marker).map_err(|e| GuardError::EnforcementFailed {
                path: path.display().to_string(),
                reason: e.to_string(),
            })?;
        }
        Ok(())
    }

    pub fn is_denied(path: &Path) -> bool {
        marker_path(path).exists()
    }

    pub fn health(path: &Path) -> ProtectionHealth {
        let denied = is_denied(path);
        ProtectionHealth {
            exists: path.exists(),
            content_deny: denied,
            metadata_deny: denied,
            mic_high_no_write_up: denied,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn health_requires_all_layers() {
        let health = ProtectionHealth {
            exists: true,
            content_deny: true,
            metadata_deny: true,
            mic_high_no_write_up: false,
        };
        assert!(!health.healthy());
    }

    #[test]
    fn health_is_healthy_when_all_layers_are_present() {
        let health = ProtectionHealth {
            exists: true,
            content_deny: true,
            metadata_deny: true,
            mic_high_no_write_up: true,
        };
        assert!(health.healthy());
    }

    #[test]
    fn missing_file_is_unhealthy() -> Result<(), Box<dyn Error>> {
        let health = verify_ace(Path::new("__agentguard_missing_file__"))?;
        assert!(!health.exists);
        assert!(!health.healthy());
        Ok(())
    }

    #[test]
    fn unprotected_existing_file_is_unhealthy() -> Result<(), Box<dyn Error>> {
        let file = tempfile::NamedTempFile::new()?;
        let health = verify_ace(file.path())?;

        assert!(health.exists);
        assert!(!health.healthy());
        Ok(())
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "requires permission to write a mandatory integrity label"]
    fn apply_and_verify_multilayer_protection() -> Result<(), Box<dyn Error>> {
        let file = tempfile::NamedTempFile::new()?;

        apply_deny_ace(file.path())?;
        let health = verify_ace(file.path())?;

        assert!(health.healthy(), "{health:?}");
        remove_deny_ace(file.path())?;
        Ok(())
    }
}
