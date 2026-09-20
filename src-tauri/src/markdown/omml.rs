//! Conservative MathML → native Office Math. Unsupported constructs use a PNG fallback.
use quick_xml::{events::Event, Reader};

#[derive(Default)]
struct Node {
    name: String,
    text: String,
    children: Vec<Node>,
    attrs: Vec<(String, String)>,
}
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
fn attr<'a>(node: &'a Node, key: &str) -> Option<&'a str> {
    node.attrs
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}
fn run(text: &str, plain: bool) -> String {
    format!("<m:r>{}<w:rPr><w:rFonts w:ascii=\"Cambria Math\" w:hAnsi=\"Cambria Math\"/></w:rPr><m:t xml:space=\"preserve\">{}</m:t></m:r>", if plain { "<m:rPr><m:sty m:val=\"p\"/></m:rPr>" } else { "" }, escape(text))
}
fn children(node: &Node) -> Option<String> {
    node.children
        .iter()
        .map(convert)
        .collect::<Option<Vec<_>>>()
        .map(|v| v.join(""))
}
fn operand(name: &str, value: String) -> String {
    format!("<m:{name}>{value}</m:{name}>")
}
fn convert(node: &Node) -> Option<String> {
    let c = &node.children;
    let n = |i| c.get(i).and_then(convert);
    Some(match node.name.as_str() {
        "math" | "mrow" | "TeXAtom" => children(node)?,
        // MathJax commonly wraps layout in mstyle; unimplemented size/color variants fall back.
        "mstyle"
            if node
                .attrs
                .iter()
                .all(|(k, _)| matches!(k.as_str(), "displaystyle" | "scriptlevel")) =>
        {
            children(node)?
        }
        "semantics" => n(0)?,
        "mi" => {
            let variant = attr(node, "mathvariant");
            if variant.is_some_and(|v| !matches!(v, "normal" | "italic" | "bold" | "bold-italic")) {
                return None;
            }
            let mut r = run(&node.text, variant == Some("normal"));
            if matches!(variant, Some("bold" | "bold-italic")) {
                r = r.replace("<w:rPr>", "<w:rPr><w:b/>");
            }
            r
        }
        "mn" | "mo" | "mtext" => run(&node.text, true),
        "mspace" => run(" ", true),
        "mfrac" if c.len() == 2 => {
            if attr(node, "bevelled") == Some("true") {
                return None;
            }
            let props = if attr(node, "linethickness") == Some("0") {
                "<m:fPr><m:type m:val=\"noBar\"/></m:fPr>"
            } else {
                ""
            };
            format!(
                "<m:f>{props}{}{}</m:f>",
                operand("num", n(0)?),
                operand("den", n(1)?)
            )
        }
        "msup" if c.len() == 2 => format!(
            "<m:sSup>{}{}</m:sSup>",
            operand("e", n(0)?),
            operand("sup", n(1)?)
        ),
        "msub" if c.len() == 2 => format!(
            "<m:sSub>{}{}</m:sSub>",
            operand("e", n(0)?),
            operand("sub", n(1)?)
        ),
        "msubsup" if c.len() == 3 => format!(
            "<m:sSubSup>{}{}{}</m:sSubSup>",
            operand("e", n(0)?),
            operand("sub", n(1)?),
            operand("sup", n(2)?)
        ),
        "msqrt" => format!(
            "<m:rad><m:radPr><m:degHide m:val=\"1\"/></m:radPr><m:deg/>{}</m:rad>",
            operand("e", children(node)?)
        ),
        "mroot" if c.len() == 2 => format!(
            "<m:rad>{}{}</m:rad>",
            operand("deg", n(1)?),
            operand("e", n(0)?)
        ),
        "mover" if c.len() == 2 => {
            if attr(node, "accent") == Some("true") && c[1].text.chars().count() == 1 {
                format!(
                    "<m:acc><m:accPr><m:chr m:val=\"{}\"/></m:accPr>{}</m:acc>",
                    escape(&c[1].text),
                    operand("e", n(0)?)
                )
            } else {
                format!(
                    "<m:limUpp>{}{}</m:limUpp>",
                    operand("e", n(0)?),
                    operand("lim", n(1)?)
                )
            }
        }
        "munder" if c.len() == 2 => format!(
            "<m:limLow>{}{}</m:limLow>",
            operand("e", n(0)?),
            operand("lim", n(1)?)
        ),
        "munderover" if c.len() == 3 => format!(
            "<m:limUpp><m:e><m:limLow>{}{}</m:limLow></m:e>{}</m:limUpp>",
            operand("e", n(0)?),
            operand("lim", n(1)?),
            operand("lim", n(2)?)
        ),
        "mtable" => {
            let mut rows = String::new();
            let columns = c.first()?.children.len();
            if columns == 0
                || c.iter()
                    .any(|row| row.name != "mtr" || row.children.len() != columns)
            {
                return None;
            }
            for row in c {
                rows.push_str("<m:mr>");
                for cell in &row.children {
                    if cell.name != "mtd" {
                        return None;
                    }
                    rows.push_str(&operand("e", children(cell)?));
                }
                rows.push_str("</m:mr>");
            }
            format!("<m:m><m:mPr><m:mcs><m:mc><m:mcPr><m:count m:val=\"{columns}\"/><m:mcJc m:val=\"center\"/></m:mcPr></m:mc></m:mcs></m:mPr>{rows}</m:m>")
        }
        "mfenced" => format!(
            "<m:d><m:dPr><m:begChr m:val=\"{}\"/><m:endChr m:val=\"{}\"/></m:dPr>{}</m:d>",
            escape(attr(node, "open").unwrap_or("(")),
            escape(attr(node, "close").unwrap_or(")")),
            operand("e", children(node)?)
        ),
        _ => return None,
    })
}

pub fn from_mathml(xml: &str) -> Option<String> {
    if xml.len() > 512_000 {
        return None;
    }
    let mut reader = Reader::from_str(xml);
    reader.config_mut().expand_empty_elements = true;
    let mut stack: Vec<Node> = Vec::new();
    let mut root = None;
    loop {
        match reader.read_event().ok()? {
            Event::Start(e) | Event::Empty(e) => {
                // Empty nodes are handled by XML expansion in the reader configuration below.
                let name = String::from_utf8(e.local_name().as_ref().to_vec()).ok()?;
                let mut attrs = Vec::new();
                for a in e.attributes() {
                    let a = a.ok()?;
                    attrs.push((
                        String::from_utf8(a.key.as_ref().to_vec()).ok()?,
                        a.decode_and_unescape_value(reader.decoder())
                            .ok()?
                            .into_owned(),
                    ));
                }
                stack.push(Node {
                    name,
                    attrs,
                    ..Default::default()
                });
                if stack.len() > 64 {
                    return None;
                }
            }
            Event::Text(e) => {
                stack.last_mut()?.text.push_str(&e.unescape().ok()?);
            }
            Event::End(_) => {
                let node = stack.pop()?;
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                } else if root.replace(node).is_some() {
                    return None;
                }
            }
            Event::Decl(_) => {}
            Event::DocType(_) | Event::PI(_) => return None,
            Event::Eof => break,
            _ => {}
        }
    }
    let root = root?;
    if root.name != "math" || !stack.is_empty() {
        return None;
    }
    Some(format!("<m:oMath>{}</m:oMath>", convert(&root)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_fraction_roots_scripts_and_matrix() {
        let xml = r#"<math xmlns="http://www.w3.org/1998/Math/MathML"><mfrac><msup><mi>x</mi><mn>2</mn></msup><msqrt><mi>y</mi></msqrt></mfrac><mtable><mtr><mtd><mn>1</mn></mtd><mtd><mn>2</mn></mtd></mtr></mtable></math>"#;
        let result = from_mathml(xml).unwrap();
        for tag in [
            "<m:oMath>",
            "<m:f>",
            "<m:sSup>",
            "<m:rad>",
            "<m:m>",
            "<m:num>",
            "<m:den>",
        ] {
            assert!(result.contains(tag), "{tag}");
        }
        assert!(from_mathml(
            "<math><mi>x</mi><mspace width=\"0.2em\"/><mo>&lt;</mo><mn>2</mn></math>"
        )
        .unwrap()
        .contains("&lt;"));
    }
    #[test]
    fn unsupported_or_untrusted_math_uses_image_fallback() {
        assert!(
            from_mathml("<math><menclose notation=\"box\"><mi>x</mi></menclose></math>").is_none()
        );
        assert!(from_mathml("<math><mi mathvariant=\"double-struck\">R</mi></math>").is_none());
        assert!(from_mathml(
            "<!DOCTYPE math [<!ENTITY x SYSTEM 'file:///etc/passwd'>]><math>&x;</math>"
        )
        .is_none());
        assert!(from_mathml("<math><mfrac><mi>x</mi></mfrac></math>").is_none());
    }
}
