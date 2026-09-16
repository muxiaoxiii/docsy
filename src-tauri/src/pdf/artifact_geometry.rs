use super::*;
use crate::pdf::qpdf_stream::{QpdfBox, QpdfEditableStream, QpdfIndexedPage, QpdfObjectIndex};

type Matrix = [f64; 6];
const IDENTITY: Matrix = [1., 0., 0., 1., 0., 0.];

fn multiply(left: Matrix, right: Matrix) -> Matrix {
    [
        left[0] * right[0] + left[2] * right[1],
        left[1] * right[0] + left[3] * right[1],
        left[0] * right[2] + left[2] * right[3],
        left[1] * right[2] + left[3] * right[3],
        left[0] * right[4] + left[2] * right[5] + left[4],
        left[1] * right[4] + left[3] * right[5] + left[5],
    ]
}

fn band(bounds: QpdfBox, matrix: Matrix, page: &QpdfIndexedPage) -> Option<&'static str> {
    let crop = page.page_box?;
    let width = f64::from(crop.x1 - crop.x0);
    let height = f64::from(crop.y1 - crop.y0);
    if width <= 0. || height <= 0. {
        return None;
    }
    let mut fractions = Vec::new();
    for (horizontal, vertical) in [
        (bounds.x0, bounds.y0),
        (bounds.x0, bounds.y1),
        (bounds.x1, bounds.y0),
        (bounds.x1, bounds.y1),
    ] {
        let visible_x =
            matrix[0] * f64::from(horizontal) + matrix[2] * f64::from(vertical) + matrix[4]
                - f64::from(crop.x0);
        let visible_y =
            matrix[1] * f64::from(horizontal) + matrix[3] * f64::from(vertical) + matrix[5]
                - f64::from(crop.y0);
        if !visible_x.is_finite()
            || !visible_y.is_finite()
            || visible_x < 0.
            || visible_x > width
            || visible_y < 0.
            || visible_y > height
        {
            return None;
        }
        fractions.push(match page.rotation {
            0 => visible_y / height,
            90 => 1. - visible_x / width,
            180 => 1. - visible_y / height,
            270 => visible_x / width,
            _ => return None,
        });
    }
    if fractions.iter().all(|value| *value <= 0.2) {
        Some("footer")
    } else if fractions.iter().all(|value| *value >= 0.8) {
        Some("header")
    } else {
        None
    }
}

#[derive(Default)]
struct Evidence {
    regions: Vec<Option<&'static str>>,
}

pub(super) fn correct_regions(
    index: &QpdfObjectIndex,
    streams: &BTreeMap<String, QpdfEditableStream>,
    page: &QpdfIndexedPage,
    inspection: &mut HeaderFooterArtifactInspection,
) {
    let mut evidence = BTreeMap::new();
    let mut matrix = IDENTITY;
    let mut stack = Vec::new();
    for reference in &page.contents {
        walk(
            index,
            streams,
            page,
            reference,
            &page.xobjects,
            &mut matrix,
            &mut stack,
            &mut BTreeSet::new(),
            &mut evidence,
        );
    }
    for occurrence in &mut inspection.occurrences {
        let Some(reference) = &occurrence.object_ref else {
            continue;
        };
        let Some(found) = evidence.get(&(reference.clone(), occurrence.operation_index)) else {
            continue;
        };
        let Some(Some(region)) = found.regions.first() else {
            continue;
        };
        if found.regions.iter().all(|value| *value == Some(*region)) {
            occurrence.region = region;
        }
    }
    inspection.header_count = inspection
        .occurrences
        .iter()
        .filter(|item| item.region == "header")
        .count();
    inspection.footer_count = inspection
        .occurrences
        .iter()
        .filter(|item| item.region == "footer")
        .count();
}

#[allow(clippy::too_many_arguments)]
fn walk(
    index: &QpdfObjectIndex,
    streams: &BTreeMap<String, QpdfEditableStream>,
    page: &QpdfIndexedPage,
    reference: &str,
    xobjects: &BTreeMap<String, String>,
    matrix: &mut Matrix,
    stack: &mut Vec<Matrix>,
    visited: &mut BTreeSet<String>,
    evidence: &mut BTreeMap<(String, usize), Evidence>,
) {
    if visited.len() >= 128 || !visited.insert(reference.to_string()) {
        return;
    }
    let Some(stream) = streams.get(reference) else {
        visited.remove(reference);
        return;
    };
    let mut active = Vec::new();
    for (position, operation) in stream.operations.iter().enumerate() {
        match operation.operator.as_str() {
            "BDC" | "BMC" => active.push(position),
            "EMC" => {
                active.pop();
            }
            "q" => stack.push(*matrix),
            "Q" => {
                if let Some(saved) = stack.pop() {
                    *matrix = saved;
                }
            }
            "cm" => {
                let values: Option<Vec<f64>> = operation
                    .operands
                    .iter()
                    .map(|value| value.as_float().ok().map(f64::from))
                    .collect();
                if let Some(values) = values {
                    if let Ok(transform) = <Vec<f64> as TryInto<Matrix>>::try_into(values) {
                        *matrix = multiply(*matrix, transform);
                    }
                }
            }
            "Do" => {
                let target = operation
                    .operands
                    .first()
                    .and_then(name_bytes)
                    .and_then(|name| std::str::from_utf8(name).ok())
                    .and_then(|name| xobjects.get(name));
                let bounds = target.and_then(|target| index.form_box(target));
                let mut transform = multiply(
                    *matrix,
                    target
                        .and_then(|target| index.form_matrix(target))
                        .unwrap_or(IDENTITY),
                );
                let region = bounds.and_then(|bounds| band(bounds, transform, page));
                for start in &active {
                    evidence
                        .entry((reference.to_string(), *start))
                        .or_default()
                        .regions
                        .push(region);
                }
                if let Some(target) = target.filter(|target| index.is_form(target)) {
                    walk(
                        index,
                        streams,
                        page,
                        target,
                        &index.form_xobjects(target),
                        &mut transform,
                        &mut Vec::new(),
                        visited,
                        evidence,
                    );
                }
            }
            "Tj" | "TJ" | "'" | "\"" | "S" | "s" | "f" | "F" | "f*" | "B" | "B*" | "b" | "b*"
            | "BI" => {
                for start in &active {
                    evidence
                        .entry((reference.to_string(), *start))
                        .or_default()
                        .regions
                        .push(None);
                }
            }
            _ => {}
        }
    }
    visited.remove(reference);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires DOCSY_ARTIFACT_FIXTURE and qpdf"]
    fn real_mislabeled_forms_are_corrected_and_footer_only_deleted() {
        let input = PathBuf::from(std::env::var("DOCSY_ARTIFACT_FIXTURE").expect("fixture path"));
        let before = inspect_meaningful_header_footer_artifacts(&input, 0).unwrap();
        let pages =
            super::super::super::qpdf::page_count(&input.to_string_lossy()).unwrap() as usize;
        assert_eq!(before.header_count, pages);
        assert_eq!(before.footer_count, pages);
        let (output, stats) = edit_header_footer_artifacts_to_temp(
            &input.to_string_lossy(),
            &HeaderFooterArtifactEditPlan {
                remove_footer: true,
                ..Default::default()
            },
        )
        .unwrap()
        .expect("edited output");
        assert_eq!(stats.removed_footer, pages);
        assert_eq!(stats.removed_header, 0);
        let after = inspect_meaningful_header_footer_artifacts(&output, 0).unwrap();
        assert_eq!(after.header_count, pages);
        assert_eq!(after.footer_count, 0);
        std::fs::remove_file(output).unwrap();
    }

    #[test]
    fn classifies_visible_bands_with_rotation_and_crop_origin() {
        let bounds = QpdfBox {
            x0: 0.,
            y0: 0.,
            x1: 60.,
            y1: 12.,
        };
        let mut page = QpdfIndexedPage {
            number: 1,
            contents: vec![],
            fonts: BTreeMap::new(),
            xobjects: BTreeMap::new(),
            properties: Dictionary::new(),
            page_box: Some(QpdfBox {
                x0: 10.,
                y0: 20.,
                x1: 610.,
                y1: 820.,
            }),
            rotation: 0,
        };
        assert_eq!(
            band(bounds, [1., 0., 0., 1., 270., 54.], &page),
            Some("footer")
        );
        assert_eq!(
            band(bounds, [1., 0., 0., 1., 270., 790.], &page),
            Some("header")
        );
        assert_eq!(band(bounds, [1., 0., 0., 1., 270., 400.], &page), None);
        page.rotation = 180;
        assert_eq!(
            band(bounds, [1., 0., 0., 1., 270., 54.], &page),
            Some("header")
        );
        page.rotation = 270;
        assert_eq!(
            band(bounds, [0., -1., 1., 0., 44., 350.], &page),
            Some("footer")
        );
        page.rotation = 90;
        assert_eq!(
            band(bounds, [0., -1., 1., 0., 44., 350.], &page),
            Some("header")
        );
    }

    #[test]
    fn accumulated_form_transform_matches_direct_transform() {
        let translated = [1., 0., 0., 1., 10., 20.];
        let rotated = [0., -1., 1., 0., 34., 333.];
        assert_eq!(multiply(translated, rotated), [0., -1., 1., 0., 44., 353.]);
        assert_eq!(multiply(rotated, IDENTITY), rotated);
    }
}
