//! Non-negative effective budgets and component comparisons.
//!
//! All arithmetic is checked. A reserve is subtracted only when the available
//! quantity is known; a missing observation never becomes an effective budget of
//! zero, and a subtraction is never allowed to wrap.

use ghostram_core::quantity::Bytes;
use serde::{Deserialize, Serialize};

use crate::evidence::Evidence;

/// An available quantity, a configured reserve, and the non-negative effective
/// budget between them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Budget {
    /// Observed or derived available quantity.
    pub available: Evidence<Bytes>,
    /// Configured reserve.
    pub reserve: Evidence<Bytes>,
    /// Available minus reserve, or unknown when either input is unknown.
    pub effective: Evidence<Bytes>,
    /// Whether a positive effective budget is available.
    pub usable: bool,
    /// Why the budget is not usable, when applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl Budget {
    /// Builds a budget without allowing a negative effective value.
    #[must_use]
    pub fn new(available: Evidence<Bytes>, reserve: Evidence<Bytes>) -> Self {
        let Some(available_value) = available.value().copied() else {
            let reason = available
                .reason()
                .unwrap_or("available memory is unknown")
                .to_string();
            return Self {
                available,
                reserve,
                effective: Evidence::unknown(reason.clone()),
                usable: false,
                reason: Some(reason),
            };
        };
        let Some(reserve_value) = reserve.value().copied() else {
            let reason = reserve
                .reason()
                .unwrap_or("the configured reserve is unknown")
                .to_string();
            return Self {
                available,
                reserve,
                effective: Evidence::unknown(reason.clone()),
                usable: false,
                reason: Some(reason),
            };
        };

        if available_value.as_u64() <= reserve_value.as_u64() {
            let reason = format!(
                "available {available_value} does not exceed the configured reserve {reserve_value}; no usable budget remains"
            );
            return Self {
                available,
                reserve,
                effective: Evidence::derived(Bytes::ZERO, reason.clone()),
                usable: false,
                reason: Some(reason),
            };
        }

        let effective = available_value.as_u64() - reserve_value.as_u64();
        Self {
            available,
            reserve,
            effective: Evidence::derived(
                Bytes::new(effective),
                "available minus the configured reserve",
            ),
            usable: true,
            reason: None,
        }
    }
}

/// Comparison of modeled components against one effective budget.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetComparison {
    /// The budget being compared.
    pub budget: Budget,
    /// Sum of the modeled components assigned to this resource.
    pub estimated_components: Evidence<Bytes>,
    /// `None` when either the components or the effective budget are unknown.
    pub within_budget: Option<bool>,
    /// Effective budget minus components, when components are within budget.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headroom: Option<Evidence<Bytes>>,
    /// Components minus effective budget, when components exceed budget.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shortfall: Option<Evidence<Bytes>>,
}

impl BudgetComparison {
    /// Compares known modeled components with a known effective budget.
    #[must_use]
    pub fn new(budget: Budget, estimated_components: Evidence<Bytes>) -> Self {
        let (Some(effective), Some(components)) = (
            budget.effective.value().copied(),
            estimated_components.value().copied(),
        ) else {
            return Self {
                budget,
                estimated_components,
                within_budget: None,
                headroom: None,
                shortfall: None,
            };
        };

        if components.as_u64() <= effective.as_u64() {
            let headroom = effective.as_u64() - components.as_u64();
            Self {
                budget,
                estimated_components,
                within_budget: Some(true),
                headroom: Some(Evidence::derived(
                    Bytes::new(headroom),
                    "effective budget minus modeled components",
                )),
                shortfall: None,
            }
        } else {
            let shortfall = components.as_u64() - effective.as_u64();
            Self {
                budget,
                estimated_components,
                within_budget: Some(false),
                headroom: None,
                shortfall: Some(Evidence::derived(
                    Bytes::new(shortfall),
                    "modeled components minus effective budget",
                )),
            }
        }
    }
}

/// Sums modeled components with checked arithmetic.
///
/// Any unknown component makes the sum unknown; the sum is never treated as
/// zero. An overflow also makes the sum unknown rather than wrapping.
#[must_use]
pub fn sum_components(components: &[&Evidence<Bytes>], description: &str) -> Evidence<Bytes> {
    let mut total = 0_u64;
    for component in components {
        let Some(value) = component.value().copied() else {
            let reason = component
                .reason()
                .unwrap_or("a modeled component is unknown")
                .to_string();
            return Evidence::unknown(format!("cannot sum modeled components: {reason}"));
        };
        let Some(next) = total.checked_add(value.as_u64()) else {
            return Evidence::unknown(
                "checked modeled-component arithmetic overflowed u64".to_string(),
            );
        };
        total = next;
    }
    Evidence::estimated(Bytes::new(total), description)
}

#[cfg(test)]
mod tests {
    use ghostram_core::quantity::Bytes;

    use super::{Budget, BudgetComparison, sum_components};
    use crate::evidence::Evidence;

    fn mib(value: u64) -> Bytes {
        Bytes::from_mib(value).unwrap()
    }

    #[test]
    fn subtracts_the_reserve_with_checked_arithmetic() {
        let budget = Budget::new(
            Evidence::observed(mib(10_000)),
            Evidence::product_default(mib(1536), "policy"),
        );
        assert!(budget.usable);
        assert_eq!(budget.effective.value(), Some(&mib(8464)));
        assert_eq!(
            budget.effective.class(),
            crate::evidence::EvidenceClass::Derived
        );
        assert!(budget.reason.is_none());
    }

    #[test]
    fn reserves_at_or_above_availability_produce_a_non_negative_unusable_budget() {
        for available in [0, 1024] {
            let budget = Budget::new(
                Evidence::observed(mib(available)),
                Evidence::product_default(mib(1536), "policy"),
            );
            assert!(!budget.usable);
            assert_eq!(budget.effective.value(), Some(&Bytes::ZERO));
            assert!(budget.reason.as_ref().unwrap().contains("does not exceed"));
        }
    }

    #[test]
    fn unknown_availability_stays_unknown_instead_of_becoming_zero() {
        let budget = Budget::new(
            Evidence::unknown("MemAvailable was not reported"),
            Evidence::product_default(mib(1536), "policy"),
        );
        assert!(!budget.usable);
        assert!(budget.effective.value().is_none());
        assert_eq!(
            budget.reason.as_deref(),
            Some("MemAvailable was not reported")
        );
    }

    #[test]
    fn compares_equal_components_as_within_budget_with_zero_headroom() {
        let comparison = BudgetComparison::new(
            Budget::new(
                Evidence::observed(mib(4096)),
                Evidence::product_default(mib(1024), "policy"),
            ),
            Evidence::estimated(mib(3072), "sum"),
        );
        assert_eq!(comparison.within_budget, Some(true));
        assert_eq!(
            comparison.headroom.as_ref().unwrap().value(),
            Some(&Bytes::ZERO)
        );
        assert!(comparison.shortfall.is_none());
    }

    #[test]
    fn reports_headroom_and_shortfall_boundaries() {
        let within = BudgetComparison::new(
            Budget::new(
                Evidence::observed(mib(4096)),
                Evidence::product_default(mib(1024), "policy"),
            ),
            Evidence::estimated(mib(2048), "sum"),
        );
        assert_eq!(within.within_budget, Some(true));
        assert_eq!(within.headroom.unwrap().value(), Some(&mib(1024)));

        let exceed = BudgetComparison::new(
            Budget::new(
                Evidence::observed(mib(4096)),
                Evidence::product_default(mib(1024), "policy"),
            ),
            Evidence::estimated(mib(4096), "sum"),
        );
        assert_eq!(exceed.within_budget, Some(false));
        assert_eq!(exceed.shortfall.unwrap().value(), Some(&mib(1024)));
        assert!(exceed.headroom.is_none());
    }

    #[test]
    fn unknown_components_produce_an_unknown_comparison() {
        let comparison = BudgetComparison::new(
            Budget::new(
                Evidence::observed(mib(4096)),
                Evidence::product_default(mib(1024), "policy"),
            ),
            Evidence::unknown("weights are unavailable"),
        );
        assert_eq!(comparison.within_budget, None);
        assert!(comparison.headroom.is_none());
        assert!(comparison.shortfall.is_none());
    }

    #[test]
    fn sums_components_only_when_every_value_is_known() {
        let one = Evidence::estimated(mib(1), "one");
        let two = Evidence::estimated(mib(2), "two");
        let sum = sum_components(&[&one, &two], "sum");
        assert_eq!(sum.value(), Some(&mib(3)));
        assert_eq!(sum.class(), crate::evidence::EvidenceClass::Estimated);

        let unknown: Evidence<Bytes> = Evidence::unknown("missing KV cache");
        let sum = sum_components(&[&one, &unknown], "sum");
        assert!(sum.value().is_none());
        assert!(sum.reason().unwrap().contains("missing KV cache"));
    }

    #[test]
    fn sum_overflow_is_unknown_and_does_not_wrap() {
        let huge = Evidence::estimated(Bytes::new(u64::MAX), "huge");
        let one = Evidence::estimated(Bytes::new(1), "one");
        let sum = sum_components(&[&huge, &one], "sum");
        assert!(sum.value().is_none());
        assert!(sum.reason().unwrap().contains("overflowed"));
    }

    #[test]
    fn budget_round_trips_through_json() {
        let comparison = BudgetComparison::new(
            Budget::new(
                Evidence::observed(mib(4096)),
                Evidence::explicit_input(mib(1024)),
            ),
            Evidence::estimated(mib(2048), "sum"),
        );
        let encoded = serde_json::to_string(&comparison).unwrap();
        let decoded: BudgetComparison = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, comparison);
    }
}
