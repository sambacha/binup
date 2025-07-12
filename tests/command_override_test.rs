//! Legacy Julia-specific override tests
//!
//! This file contains tests that were specific to Julia execution through juliaup.
//! These tests have been disabled as part of the refactoring to GUP (Generic Updater/Installer Program).
//! New override tests would need to be written for the generic GUP architecture.

#[cfg(test)]
mod legacy_julia_tests {
    // These tests were disabled during the juliaup -> gup refactoring
    // They tested Julia-specific functionality like:
    // - Installing Julia versions (1.6.7, 1.8.5)
    // - Setting directory overrides for Julia
    // - Executing Julia with version overrides
    // - Testing Julia VERSION output

    // For GUP, similar tests would need to be written that:
    // - Test with generic project metadata
    // - Use mock projects instead of real Julia versions
    // - Test override functionality in a project-agnostic way
}
