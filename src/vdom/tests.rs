use crate::vdom::{
    Attribute, Document, ElementData, Node, NodeData, QualName, StrTendril,
    filter,
    filter::{Action, FilterChain, TreeFilter},
    html::{a, t},
};

#[test]
#[cfg(target_pointer_width = "64")]
fn size_of() {
    use std::mem::size_of;
    assert_eq!(size_of::<Node>(), 88);
    assert_eq!(size_of::<NodeData>(), 64);
    assert_eq!(size_of::<ElementData>(), 56);
    assert_eq!(size_of::<Attribute>(), 48);
    assert_eq!(size_of::<Vec<Attribute>>(), 24);
    assert_eq!(size_of::<QualName>(), 32);
    assert_eq!(size_of::<StrTendril>(), 16);
}

#[test]
fn empty_document() {
    let doc = Document::new();
    assert_eq!(None, doc.root_element_ref(), "no root Element");
    assert_eq!(1, doc.nodes().count(), "one Document node");
}

#[test]
fn one_element() {
    let mut doc = Document::new();
    let element = Node::new(NodeData::Element(
        ElementData {
            name: QualName::new(None, ns!(), "one".into()),
            attrs: vec![]
        }
    ));
    let id = doc.append_child(Document::DOCUMENT_NODE_ID, element);

    assert!(doc.root_element_ref().is_some(), "pushed root Element");
    assert_eq!(id, doc.root_element_ref().unwrap().id);
    assert_eq!(2, doc.nodes().count(), "root + 1 element");
}

struct StrikeRemoveFilter;

impl TreeFilter for StrikeRemoveFilter {
    fn filter(&self, node: &mut Node) -> Action {
        if node.is_elem(t::STRIKE) {
            Action::Detach
        } else {
            Action::Continue
        }
    }
}

struct StrikeFoldFilter;

impl TreeFilter for StrikeFoldFilter {
    fn filter(&self, node: &mut Node) -> Action {
        if node.is_elem(t::STRIKE) {
            Action::Fold
        } else {
            Action::Continue
        }
    }
}

#[test]
fn test_fold_filter() {
    let mut doc = Document::parse_html(
        "<div>foo <strike><i>bar</i>s</strike> baz</div>"
            .as_bytes()
    );
    doc.filter(&StrikeFoldFilter {});
    assert_eq!(
        "<html><head></head><body>\
         <div>foo <i>bar</i>s baz</div>\
         </body></html>",
        doc.to_string()
    );
}

#[test]
fn test_remove_filter() {
    let mut doc = Document::parse_html(
        "<div>foo <strike><i>bar</i>s</strike> baz</div>"
            .as_bytes()
    );
    doc.filter(&StrikeRemoveFilter {});
    assert_eq!(
        "<html><head></head><body>\
         <div>foo  baz</div>\
         </body></html>",
        doc.to_string()
    );
}

#[test]
fn test_filter_chain() {
    let mut doc = Document::parse_html_fragment(
        "<div>foo<strike><i>bar</i>s</strike> \n\t baz</div>"
            .as_bytes()
    );
    let fltrs = FilterChain::new(vec![
        Box::new(StrikeRemoveFilter {}),
        Box::new(filter::TextNormalizer)
    ]);

    doc.filter(&fltrs);
    assert_eq!(
        "<div>foo baz</div>",
        doc.to_string()
    );
}

#[test]
fn test_xmp() {
    let doc = Document::parse_html_fragment(
        "<div>foo <xmp><i>bar</i></xmp> baz</div>"
            .as_bytes()
    );
    assert_eq!(
        "<div>foo <xmp><i>bar</i></xmp> baz</div>",
        doc.to_string()
    );

    // Currently node count is only ensured by cloning
    let doc = doc.deep_clone(doc.root_element().unwrap());
    eprintln!("the doc nodes:\n{:?}", doc);
    assert_eq!(5, doc.nodes().count() - 1);
}

#[test]
fn test_plaintext() {
    let doc = Document::parse_html_fragment(
        "<div><plaintext><i>bar baz</div>"
            .as_bytes()
    );
    // Serializer isn't aware that <plaintext> doesn't need end tags, etc.
    assert_eq!(
        "<div><plaintext><i>bar baz</div></plaintext></div>",
        doc.to_string()
    );

    // Currently node count is only ensured by cloning
    let doc = doc.deep_clone(doc.root_element().unwrap());
    eprintln!("the doc nodes:\n{:?}", doc);
    assert_eq!(3, doc.nodes().count() - 1);
}

#[test]
fn test_text_fragment() {
    let doc = Document::parse_html_fragment(
        "plain &lt; text".as_bytes()
    );
    assert_eq!(
        "<div>\
         plain &lt; text\
         </div>",
        doc.to_string()
    );

    // Currently node count is only ensured by cloning
    let doc = doc.deep_clone(doc.root_element().unwrap());
    eprintln!("the doc nodes:\n{:?}", doc);
    assert_eq!(2, doc.nodes().count() - 1);
}

#[test]
fn test_empty_tag() {
    let doc = Document::parse_html_fragment(
        "plain<wbr>text".as_bytes()
    );
    assert_eq!(
        "<div>\
         plain<wbr>text\
         </div>",
        doc.to_string()
    );

    // Currently node count is only ensured by cloning
    let doc = doc.deep_clone(doc.root_element().unwrap());
    eprintln!("the doc nodes:\n{:?}", doc);
    assert_eq!(4, doc.nodes().count() - 1);
}

#[test]
fn test_shallow_fragment() {
    let doc = Document::parse_html_fragment(
        "<b>b</b> text <i>i</i>".as_bytes()
    );
    assert_eq!(
        "<div>\
         <b>b</b> text <i>i</i>\
         </div>",
        doc.to_string()
    );

    // Currently node count is only ensured by cloning
    let doc = doc.deep_clone(doc.root_element().unwrap());
    eprintln!("the doc nodes:\n{:?}", doc);
    assert_eq!(6, doc.nodes().count() - 1);
}

#[test]
fn test_empty_fragment() {
    let doc = Document::parse_html_fragment("".as_bytes());
    eprintln!("the doc nodes:\n{:?}", doc);
    assert_eq!("<div></div>", doc.to_string());
}

#[test]
fn test_deep_clone() {
    let doc = Document::parse_html(
        "<div>foo <a href=\"link\"><i>bar</i>s</a> baz</div>\
         <div>sibling</div>"
            .as_bytes()
    );

    let doc = doc.deep_clone(doc.root_element().expect("root"));
    assert_eq!(
        "<html><head></head><body>\
           <div>foo <a href=\"link\"><i>bar</i>s</a> baz</div>\
           <div>sibling</div>\
         </body></html>",
        doc.to_string()
    );
}

#[test]
fn test_filter() {
    let doc = Document::parse_html(
        "<p>1</p>\
         <div>\
           fill\
           <p>2</p>\
           <p>3</p>\
           <div>\
             <p>4</p>\
             <i>fill</i>\
           </div>\
         </div>"
            .as_bytes()
    );

    let root = doc.root_element_ref().expect("root");
    let body = root.find(|n| n.is_elem(t::BODY)).expect("body");
    let f1: Vec<_> = body
        .filter(|n| n.is_elem(t::P))
        .map(|n| n.text().unwrap().to_string())
        .collect();

    assert_eq!(f1, vec!["1"]);
}

#[test]
fn test_filter_r() {
    let doc = Document::parse_html_fragment(
        "<p>1</p>\
         <div>\
           fill\
           <p>2</p>\
           <p>3</p>\
           <div>\
             <p>4</p>\
             <i>fill</i>\
           </div>\
         </div>"
            .as_bytes()
    );

    let root = doc.root_element_ref().expect("root");

    assert_eq!("1fill234fill", root.text().unwrap().to_string());

    let f1: Vec<_> = root
        .filter_r(|n| n.is_elem(t::P))
        .map(|n| n.text().unwrap().to_string())
        .collect();

    assert_eq!(f1, vec!["1", "2", "3", "4"]);
}

#[test]
fn test_meta_content_type() {
    let doc = Document::parse_html(
        r####"
<html xmlns="http://www.w3.org/1999/xhtml">
 <head>
  <meta charset='UTF-8'/>
  <META http-equiv=" CONTENT-TYPE" content="text/html; charset=utf-8"/>
  <title>Iūdex</title>
 </head>
 <body>
  <p>Iūdex test.</p>
 </body>
</html>"####
            .as_bytes()
    );
    let root = doc.root_element_ref().expect("root");
    let head = root.find(|n| n.is_elem(t::HEAD)).expect("head");
    let mut found = false;
    for m in head.filter(|n| n.is_elem(t::META)) {
        if let Some(a) = m.attr(a::CHARSET) {
            eprintln!("meta charset: {}", a);
        } else if let Some(a) = m.attr(a::HTTP_EQUIV) {
            // FIXME: Parser doesn't normalize whitespace in
            // attributes. Need to trim.
            if a.as_ref().trim().eq_ignore_ascii_case("Content-Type") {
                if let Some(a) = m.attr(a::CONTENT) {
                    let ctype = a.as_ref().trim();
                    eprintln!("meta content-type: {}", ctype);
                    assert_eq!("text/html; charset=utf-8", ctype);
                    found = true;
                }
            }
        }
    }
    assert!(found);
}