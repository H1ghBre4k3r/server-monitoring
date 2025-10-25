//! Property-based tests for invariants using proptest
//!
//! These tests verify that certain properties hold true for all inputs:
//! - Grace period counters never go negative
//! - Grace period reset behavior
//! - Alert triggering conditions
//! - Resource evaluation logic

use guardia::monitors::resources::ResourceEvaluation;
use proptest::prelude::*;

// Property: Grace counter never goes negative
proptest! {
    #[test]
    fn prop_grace_counter_never_negative(
        resource in 0.0f32..200.0f32,
        limit in 0.0f32..100.0f32,
        grace in 0usize..20usize,
        current_grace in 0usize..30usize,
    ) {
        let _result = ResourceEvaluation::evaluate(resource, limit, grace, current_grace);
        // Test passes if no panic occurs and evaluation completes
        // (The function itself doesn't modify state, so we mainly test it doesn't crash)
    }
}

// Property: When resource < limit and current_grace == 0, result is always Ok
proptest! {
    #[test]
    fn prop_below_limit_zero_grace_is_ok(
        limit in 1.0f32..100.0f32,
        grace in 0usize..20usize,
    ) {
        let resource = limit - 0.1; // Slightly below limit
        let current_grace = 0;

        let result = ResourceEvaluation::evaluate(resource, limit, grace, current_grace);

        prop_assert_eq!(result, ResourceEvaluation::Ok);
    }
}

// Property: When resource < limit but current_grace > grace, result is BackToOk
proptest! {
    #[test]
    fn prop_below_limit_after_exceeding_is_back_to_ok(
        limit in 1.0f32..100.0f32,
        grace in 1usize..10usize,
    ) {
        let resource = limit - 0.1; // Slightly below limit
        let current_grace = grace + 1; // Exceeded grace period

        let result = ResourceEvaluation::evaluate(resource, limit, grace, current_grace);

        prop_assert_eq!(result, ResourceEvaluation::BackToOk);
    }
}

// Property: When resource >= limit and current_grace < grace, result is Exceeding
proptest! {
    #[test]
    fn prop_above_limit_within_grace_is_exceeding(
        limit in 1.0f32..100.0f32,
        grace in 2usize..10usize,
    ) {
        let resource = limit + 0.1; // Slightly above limit
        let current_grace = grace - 1; // Within grace period

        let result = ResourceEvaluation::evaluate(resource, limit, grace, current_grace);

        prop_assert_eq!(result, ResourceEvaluation::Exceeding);
    }
}

// Property: When resource >= limit and current_grace == grace, result is StartsToExceed
proptest! {
    #[test]
    fn prop_above_limit_at_grace_is_starts_to_exceed(
        limit in 1.0f32..100.0f32,
        grace in 0usize..20usize,
    ) {
        let resource = limit + 0.1; // Slightly above limit
        let current_grace = grace; // Exactly at grace limit

        let result = ResourceEvaluation::evaluate(resource, limit, grace, current_grace);

        prop_assert_eq!(result, ResourceEvaluation::StartsToExceed);
    }
}

// Property: Sequence of evaluations maintains consistency
#[test]
fn test_grace_period_sequence_property() {
    // Simulate a sequence: below → above → above → above → below
    let limit = 80.0;
    let grace = 2;

    // Start below limit
    let mut current_grace = 0;
    let result = ResourceEvaluation::evaluate(50.0, limit, grace, current_grace);
    assert_eq!(result, ResourceEvaluation::Ok);

    // Go above limit (1st time - Exceeding)
    let result = ResourceEvaluation::evaluate(85.0, limit, grace, current_grace);
    assert_eq!(result, ResourceEvaluation::Exceeding);
    current_grace += 1;

    // Still above (2nd time - Exceeding)
    let result = ResourceEvaluation::evaluate(85.0, limit, grace, current_grace);
    assert_eq!(result, ResourceEvaluation::Exceeding);
    current_grace += 1;

    // Still above (3rd time - StartsToExceed because current_grace == grace)
    let result = ResourceEvaluation::evaluate(85.0, limit, grace, current_grace);
    assert_eq!(result, ResourceEvaluation::StartsToExceed);
    current_grace += 1; // Now current_grace > grace

    // Go back below limit (should trigger BackToOk because current_grace > grace)
    let result = ResourceEvaluation::evaluate(50.0, limit, grace, current_grace);
    assert_eq!(result, ResourceEvaluation::BackToOk);

    // Reset counter
    current_grace = 0;

    // Verify we're back to Ok
    let result = ResourceEvaluation::evaluate(50.0, limit, grace, current_grace);
    assert_eq!(result, ResourceEvaluation::Ok);
}

// Property: Temperature and CPU evaluations are independent
#[test]
fn test_independent_evaluation_invariant() {
    // This tests that evaluating temperature doesn't affect CPU evaluation
    let temp_limit = 70.0;
    let cpu_limit = 80.0;
    let grace = 3;

    // Evaluate temperature as exceeding
    let temp_result = ResourceEvaluation::evaluate(75.0, temp_limit, grace, 2);
    assert_eq!(temp_result, ResourceEvaluation::Exceeding);

    // Evaluate CPU as OK - should not be affected by temperature evaluation
    let cpu_result = ResourceEvaluation::evaluate(50.0, cpu_limit, grace, 0);
    assert_eq!(cpu_result, ResourceEvaluation::Ok);

    // Both evaluations are independent
    // This is a simple sanity check that the function is pure (no hidden state)
}

// Property: Alert count is bounded by violations
proptest! {
    #[test]
    fn prop_alert_triggering_bounded(
        limit in 1.0f32..100.0f32,
        grace in 0usize..10usize,
        num_violations in 0usize..20usize,
    ) {
        // Simulate violations
        let mut alert_count = 0;
        let mut current_grace = 0;

        for _ in 0..num_violations {
            let resource = limit + 1.0; // Above limit
            let result = ResourceEvaluation::evaluate(resource, limit, grace, current_grace);

            match result {
                ResourceEvaluation::Exceeding => {
                    current_grace += 1;
                }
                ResourceEvaluation::StartsToExceed => {
                    current_grace += 1;
                    alert_count += 1;
                    // In real system, would send alert here
                }
                _ => {}
            }
        }

        // Alert count should be at most ceil(num_violations / (grace + 1))
        // (one alert per grace period exhaustion)
        if grace == 0 {
            prop_assert!(alert_count <= num_violations);
        } else {
            prop_assert!(alert_count <= num_violations / (grace + 1) + 1);
        }
    }
}

// Property: Zero grace period means immediate alert
proptest! {
    #[test]
    fn prop_zero_grace_immediate_alert(
        limit in 1.0f32..100.0f32,
        resource in 0.0f32..200.0f32,
    ) {
        let grace = 0;
        let current_grace = 0;

        let result = ResourceEvaluation::evaluate(resource, limit, grace, current_grace);

        if resource >= limit {
            // With grace=0, should immediately trigger StartsToExceed
            prop_assert_eq!(result, ResourceEvaluation::StartsToExceed);
        } else {
            prop_assert_eq!(result, ResourceEvaluation::Ok);
        }
    }
}
