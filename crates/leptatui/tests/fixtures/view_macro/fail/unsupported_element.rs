use leptatui_macros::view;

fn main() {
    let _ = view! {
        <Block data_id="bad">
            <Text>"bad"</Text>
        </Block>
    };
}
