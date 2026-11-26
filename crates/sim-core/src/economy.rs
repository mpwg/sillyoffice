use crate::World;

impl World {
    pub(crate) fn settle_day(&mut self) {
        let Some(rules) = self.scenario.rules else {
            return;
        };
        if !self.tick().is_multiple_of(rules.day_ticks.get()) {
            return;
        }
        let occupied_homes = self.state.households.len() as i128;
        let occupied_offices = self
            .state
            .people
            .values()
            .filter(|person| person.office.is_some())
            .count() as i128;
        let expenses: i128 = self
            .scenario
            .geometry()
            .sections
            .iter()
            .filter(|section| self.is_rented(section.id))
            .map(|section| i128::from(section.rent_cost))
            .sum();
        let rent = occupied_homes * i128::from(rules.home_income)
            + occupied_offices * i128::from(rules.office_income)
            - expenses;
        self.state.balance = (i128::from(self.state.balance) + rent)
            .clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64;
    }
}
