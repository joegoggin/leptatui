//! Pass fixture for reactive text children in `view!`.
//!
//! This binary verifies closures and supported readable signal types lower
//! into tracked terminal view boundaries.

use leptatui::prelude::*;

/// Exercises reactive text content and direct signal child expansion.
fn main() {
    Owner::new().with(|| {
        let count = RwSignal::new(String::from("7"));
        let read_count = count.read_only();
        let memo = Memo::new(move |_| count.get());
        let signal = Signal::derive(move || count.get());
        let arc_count = ArcRwSignal::new(String::from("8"));
        let arc_read_count = arc_count.read_only();
        let arc_memo = ArcMemo::new({
            let arc_count = ArcRwSignal::clone(&arc_count);
            move |_| arc_count.get()
        });

        let _: AnyView = count.into_view();
        let _: AnyView = read_count.into_view();
        let _: AnyView = memo.into_view();
        let _: AnyView = signal.into_view();
        let _: AnyView = ArcRwSignal::clone(&arc_count).into_view();
        let _: AnyView = ArcReadSignal::clone(&arc_read_count).into_view();
        let _: AnyView = ArcMemo::clone(&arc_memo).into_view();

        let _closure = view! { <Text>{move || count.get()}</Text> };
        let _rw_signal = view! { <Text>{count}</Text> };
        let _read_signal = view! { <H1>{read_count}</H1> };
        let _h2 = view! { <H2>{count}</H2> };
        let _h3 = view! { <H3>{count}</H3> };
        let _h4 = view! { <H4>{count}</H4> };
        let _h5 = view! { <H5>{count}</H5> };
        let _h6 = view! { <H6>{count}</H6> };
        let _memo = view! { <Paragraph>{memo}</Paragraph> };
        let _signal = view! { <CodeBlock language="text">{signal}</CodeBlock> };
        let _arc_rw = view! {
            <Button on_press=|| AppControl::Continue>{ArcRwSignal::clone(&arc_count)}</Button>
        };
        let _arc_read = view! {
            <Link href="https://example.com">{arc_read_count}</Link>
        };
        let _route_link = view! { <A href="/">{count}</A> };
        let _arc_memo = view! {
            <Table>
                <TableBody>
                    <TableRow>
                        <TableCell alignment=CellAlignment::Center>
                            {ArcMemo::clone(&arc_memo)}
                        </TableCell>
                    </TableRow>
                </TableBody>
            </Table>
        };
        let _container_child = view! { <Div>{count}</Div> };
    });
}
