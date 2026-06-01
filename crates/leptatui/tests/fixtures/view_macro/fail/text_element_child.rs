use leptatui_macros::view;

fn main() {
    let _ = view! {
        <Text>
            <Block>
                <Text>"bad"</Text>
            </Block>
        </Text>
    };
}
