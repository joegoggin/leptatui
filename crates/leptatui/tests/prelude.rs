use leptatui::prelude::*;

#[test]
fn prelude_exposes_reactivity_and_context() {
    Owner::new().with(|| {
        let (count, set_count) = signal(0);

        assert_eq!(count.get(), 0);

        set_count.set(1);
        set_count.update(|value| *value += 1);

        assert_eq!(count.get(), 2);

        let doubled = Memo::new(move |_| count.get() * 2);

        assert_eq!(doubled.get(), 4);

        provide_context(String::from("from prelude"));

        assert_eq!(use_context::<String>().as_deref(), Some("from prelude"));
    });
}
