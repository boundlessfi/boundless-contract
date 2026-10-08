//! Seeded random grants: budgets, milestone counts and splits, tiers, partner
//! money before and after selection, releases and forfeits in any order, then the
//! remainder returned. Every token is accounted for at the end. `GRANT_SCENARIO_RUNS=<n>` runs more seeds.

use super::*;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    fn chance(&mut self, percent: u64) -> bool {
        self.below(100) < percent
    }
}

fn runs() -> u64 {
    std::env::var("GRANT_SCENARIO_RUNS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(48)
}

#[test]
fn random_grants_account_for_every_token() {
    for seed in 0..runs() {
        run(seed);
    }
}

fn run(seed: u64) {
    let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03);
    let g = setup();

    let budget = usdc(2_000 + rng.below(10_000) as i128) + rng.below(USDC as u64) as i128;
    let n = 1 + rng.below(10) as u32;
    let k = 1 + rng.below(8) as usize;
    let split: Option<std::vec::Vec<u32>> = if n > 1 && rng.chance(50) {
        let raw: std::vec::Vec<u32> = (0..n).map(|_| 1 + rng.below(100) as u32).collect();
        let total: u32 = raw.iter().sum();
        let mut shares: std::vec::Vec<u32> = raw.iter().map(|w| w * 10_000 / total).collect();
        let short = 10_000 - shares.iter().sum::<u32>();
        let largest = (0..shares.len()).max_by_key(|&i| shares[i]).unwrap();
        shares[largest] += short;
        Some(shares)
    } else {
        None
    };
    // The smallest award whose every milestone pays at least one unit.
    let min_award: i128 = match &split {
        Some(shares) => {
            let smallest = *shares.iter().min().unwrap() as i128;
            (10_000 + smallest - 1) / smallest
        }
        None => n as i128,
    };

    let early: std::vec::Vec<(Address, i128)> = (0..rng.below(4))
        .map(|_| (g.someone(), usdc(10 + rng.below(500) as i128)))
        .collect();
    let pool = budget + early.iter().map(|(_, a)| a).sum::<i128>();

    let share = pool * (50 + rng.below(51) as i128) / 100;
    let weights: std::vec::Vec<i128> = (0..k).map(|_| 1 + rng.below(100) as i128).collect();
    let weight_sum: i128 = weights.iter().sum();
    let people: std::vec::Vec<Address> = (0..k).map(|_| g.someone()).collect();
    let awards: std::vec::Vec<i128> = weights
        .iter()
        .map(|w| (share * w / weight_sum).max(min_award))
        .collect();
    assert!(
        awards.iter().sum::<i128>() <= pool,
        "seed {seed}: plan exceeds pool"
    );

    let floors: std::vec::Vec<(u32, i128)> = awards
        .iter()
        .enumerate()
        .map(|(i, a)| (i as u32 + 1, (a / 4).max(1)))
        .collect();
    let spec = Spec::new(budget, n).floors(&floors);
    let id = g.create(match &split {
        Some(shares) => spec.split(shares),
        None => spec,
    });
    for (partner, amount) in early.iter() {
        g.contribute(id, partner, *amount);
    }

    let selection: std::vec::Vec<(Address, u32, i128)> = people
        .iter()
        .zip(awards.iter())
        .enumerate()
        .map(|(i, (p, a))| (p.clone(), i as u32 + 1, *a))
        .collect();
    g.select(id, &selection);

    let mut queue: std::vec::Vec<(usize, u32, bool)> = (0..k)
        .flat_map(|r| (0..n).map(move |m| (r, m)))
        .map(|(r, m)| (r, m, rng.chance(15)))
        .collect();
    for i in (1..queue.len()).rev() {
        let j = rng.below(i as u64 + 1) as usize;
        queue.swap(i, j);
    }

    let mut late: std::vec::Vec<(Address, i128)> = std::vec::Vec::new();
    let mut paid = std::vec![0i128; k];
    let mut settled = std::vec![0i128; k];
    let mut count = std::vec![0u32; k];
    for (step, (r, m, forfeit)) in queue.iter().enumerate() {
        if late.len() < 2 && step == queue.len() / 2 && rng.chance(50) {
            let partner = g.someone();
            let amount = usdc(10 + rng.below(200) as i128);
            g.contribute(id, &partner, amount);
            late.push((partner, amount));
        }
        count[*r] += 1;
        let expected = if count[*r] == n {
            awards[*r] - settled[*r]
        } else {
            match &split {
                Some(shares) => awards[*r] * shares[*m as usize] as i128 / 10_000,
                None => awards[*r] / n as i128,
            }
        };
        if *forfeit {
            g.forfeit(id, &people[*r], *m, expected);
        } else {
            g.pay(id, &people[*r], *m, expected);
            paid[*r] += expected;
        }
        settled[*r] += expected;
    }

    assert_eq!(g.owed(id), 0, "seed {seed}: every award settled");
    for r in 0..k {
        assert_eq!(g.balance(&people[r]), paid[r], "seed {seed}: recipient {r}");
        assert_eq!(settled[r], awards[r], "seed {seed}: award settled in full");
    }

    let partners: std::vec::Vec<(Address, i128)> =
        early.iter().chain(late.iter()).cloned().collect();
    let partner_total: i128 = partners.iter().map(|(_, a)| a).sum();
    let remaining = g.event(id).remaining_escrow;
    assert_eq!(
        remaining,
        pool + late.iter().map(|(_, a)| a).sum::<i128>() - paid.iter().sum::<i128>(),
        "seed {seed}: escrow tracks deposits minus payouts"
    );

    let mut dust = 0;
    if g.status(id) == EventStatus::Completed {
        assert_eq!(remaining, 0);
    } else {
        g.cancel(id);
        let owner_refund = if partner_total == 0 {
            remaining
        } else if remaining >= partner_total {
            for (p, a) in partners.iter() {
                assert_eq!(g.balance(p), *a, "seed {seed}: partner refunded in full");
            }
            remaining - partner_total
        } else {
            let mut refunded = 0;
            for (p, a) in partners.iter() {
                let share = a * remaining / partner_total;
                assert_eq!(g.balance(p), share, "seed {seed}: partner pro rata");
                refunded += share;
            }
            dust = remaining - refunded;
            0
        };
        assert_eq!(
            g.balance(&g.owner),
            owner_refund,
            "seed {seed}: owner refund"
        );
    }

    assert_eq!(
        g.escrow_balance(),
        dust,
        "seed {seed}: contract keeps only dust"
    );
    let deposits = budget + partner_total;
    let fees = fee_on(budget, FEE_BPS)
        + partners
            .iter()
            .map(|(_, a)| fee_on(*a, FEE_BPS))
            .sum::<i128>();
    assert_eq!(g.balance(&g.fee_account), fees, "seed {seed}: fees");
    let out = paid.iter().sum::<i128>()
        + g.balance(&g.owner)
        + partners.iter().map(|(p, _)| g.balance(p)).sum::<i128>()
        + dust;
    assert_eq!(
        out, deposits,
        "seed {seed}: every deposited token is accounted for"
    );
}
