// Copyright (C) 2013 - 2021 Tim Düsterhus
// Copyright (C) 2021 Maximilian Mader
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

/// Simple usage:
/// ```rust
/// fluent!(language, "id");
/// ```
///
/// Passing variables:
/// ```rust
/// fluent!(language, "id", { "foo": "bar", "baz": 1 });
/// ```
/// Shorthand for values that are in scope:
/// ```rust
/// let foo = "bar";
///
/// fluent!(language, "id", { foo });
///
/// // This is equivalent to:
/// fluent!(language, "id", { "foo": foo });
/// ```
///
/// The shorthand also works with nested struct values:
/// ```rust
/// fluent!(language, "id", { foo.bar.baz });
///
/// // This is equivalent to:
/// fluent!(language, "id", { "baz": foo.bar.baz });
/// ```
#[macro_export]
macro_rules! fluent {
    // Helper rule to get the last identifier of a set of nested identifiers,
    // i.e. `foo.bar.baz` => `"baz"`
    (@last_ident $ident:ident) => ( stringify!($ident) );
    (@last_ident $ident:ident . $($rest:ident).+) => (
        fluent!(@last_ident $($rest).+)
    );

    // Done
    (@object $object:ident ()) => {};

    // Take a "key": value pair
    (@object $object:ident ( $key:literal : $value:expr $(, $($tail:tt)*)? ) ) => ({
        let _ = $object.insert(
            $key.into(),
            ::serde_json::to_value(&$value).unwrap()
        );

        fluent!(@object $object ( $( $($tail)* )? ) );
    });

    // Take a (nested) shorthand
    (@object $object:ident ( $ident:ident $(. $rest:ident)* $(, $($tail:tt)*)? ) ) => ({
        let ident = fluent!(@last_ident $ident $(. $rest)*);

        let _ = $object.insert(
            ident.into(),
            ::serde_json::to_value(& $ident $(. $rest)* ).unwrap()
        );

        fluent!(@object $object ( $( $($tail)* )? ) );
    });

    // End of the macro
    (@lookup $lang:expr, $id:literal, $($args:tt)+) => ({
        let args = $($args)+;

        $crate::fluent::lookup(
            $id,
            &$lang.to_string(),
            &args
        )
        .unwrap_or_else(|_| $id.to_owned())
    });

    // The simple case without variables
    ($lang:expr, $id:literal) => (
        fluent!($lang, $id, json: { })
    );

    // Entrypoint with variables
    ($lang:expr, $id:literal, { $($tt:tt)+ }) => (
        fluent!(@lookup $lang, $id, ::serde_json::Value::Object({
            let mut object = ::serde_json::Map::new();
            fluent!(@object object ( $($tt)+ ));
            object
        }))
    );

    // JSON entrypoint, alias to `::serde_json::json`
    ($lang:expr, $id:literal, json: $($tt:tt)+) => ({
        fluent!(@lookup $lang, $id, ::serde_json::json!( $($tt)+ ))
    });

    // Fallback entrypoint
    ($lang:expr, $id:literal, $($tt:tt)+) => ({
        fluent!(@lookup $lang, $id, $($tt)+)
    });
}

#[test]
fn test_basic_syntax() {
    assert_eq!(
        fluent!(super::DEFAULT, "CARGO_TEST_STRING_1"),
        "Static String"
    );
}

#[test]
fn test_basic_parameter() {
    let foo = "bar";
    assert_eq!(
        fluent!(super::DEFAULT, "CARGO_TEST_STRING_2", { "foo": foo }),
        "foo = bar"
    );
}

#[test]
fn test_multiple_basic_parameters() {
    let foo = "bar";
    assert_eq!(
        fluent!(super::DEFAULT, "CARGO_TEST_STRING_2", { "foo": foo, "bar": "baz", "barbaz": "foobar" }),
        "foo = bar"
    );
    assert_eq!(
        fluent!(super::DEFAULT, "CARGO_TEST_STRING_3", { "foo": foo, "bar": "baz", "barbaz": "foobar" }),
        "bar = baz"
    );
    assert_eq!(
        fluent!(super::DEFAULT, "CARGO_TEST_STRING_5", { "foo": foo, "bar": "baz", "barbaz": "foobar" }),
        "foo = bar
bar = baz
barbaz = foobar"
    );
}

#[test]
fn test_scope_shorthand() {
    let foo = "bar";
    assert_eq!(
        fluent!(super::DEFAULT, "CARGO_TEST_STRING_2", { foo }),
        "foo = bar"
    );
}

#[test]
fn test_self_static_str() {
    struct Foo {
        foo: &'static str,
    }

    impl Foo {
        fn bar(self) -> String {
            fluent!(super::DEFAULT, "CARGO_TEST_STRING_2", { self.foo })
        }
    }

    assert_eq!(Foo { foo: "bar" }.bar(), "foo = bar");
}

#[test]
fn test_self_string() {
    struct Foo {
        foo: String,
    }

    impl Foo {
        fn bar(self) -> String {
            fluent!(super::DEFAULT, "CARGO_TEST_STRING_2", { self.foo })
        }
    }

    assert_eq!(
        Foo {
            foo: "bar".to_owned()
        }
        .bar(),
        "foo = bar"
    );
}

#[test]
fn test_struct_shorthand() {
    struct Foo {
        bar: &'static str,
    }

    let foo = Foo { bar: "baz" };

    assert_eq!(
        fluent!(super::DEFAULT, "CARGO_TEST_STRING_3", { foo.bar }),
        "bar = baz"
    );
}

#[test]
fn test_double_nested_struct_shorthand() {
    struct Bar {
        baz: &'static str,
    }

    struct Foo {
        bar: Bar,
    }

    let foo = Foo {
        bar: Bar { baz: "barbaz" },
    };

    assert_eq!(
        fluent!(super::DEFAULT, "CARGO_TEST_STRING_4", { foo.bar.baz }),
        "baz = barbaz"
    );
}
