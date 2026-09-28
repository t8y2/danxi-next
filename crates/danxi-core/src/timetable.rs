use serde::Serialize;
use serde_json::Value;

use crate::AppError;

/// One timetable slot: a course on a weekday, spanning lesson periods.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimetableCourse {
    pub course_name: String,
    pub room_name: Option<String>,
    pub teacher_names: Vec<String>,
    /// 1 = Monday … 7 = Sunday.
    pub weekday: u8,
    /// 1-based lesson period.
    pub start_unit: u8,
    pub end_unit: u8,
    /// 1-based week numbers in which this slot occurs.
    pub weeks: Vec<u16>,
}

/// A semester's timetable plus its Monday start date (undergrad only; the
/// postgraduate system does not expose one).
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Timetable {
    pub courses: Vec<TimetableCourse>,
    pub semester_start_date: Option<String>,
}

/// Parse the undergraduate (JWGL) `print-data` payload.
///
/// Shape: `studentTableVms[0].activities[]` with `courseName`, `room`,
/// `teachers`, `weekIndexes`, `weekday`, `startUnit`, `endUnit`.
pub fn parse_jwgl(payload: &Value) -> Result<Timetable, AppError> {
    let activities = payload
        .get("studentTableVms")
        .and_then(Value::as_array)
        .and_then(|tables| tables.first())
        .and_then(|table| table.get("activities"))
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::Upstream("课表数据格式无法解析".to_owned()))?;

    let mut courses = Vec::new();
    for activity in activities {
        let weekday = activity.get("weekday").and_then(Value::as_i64).unwrap_or(1) as u8;
        let start_unit = activity
            .get("startUnit")
            .and_then(Value::as_i64)
            .unwrap_or(1) as u8;
        let end_unit = activity
            .get("endUnit")
            .and_then(Value::as_i64)
            .unwrap_or(start_unit as i64) as u8;
        courses.push(TimetableCourse {
            course_name: activity
                .get("courseName")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            room_name: activity
                .get("room")
                .and_then(Value::as_str)
                .map(str::to_owned),
            teacher_names: activity
                .get("teachers")
                .and_then(Value::as_array)
                .map(|teachers| {
                    teachers
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_default(),
            weekday,
            start_unit,
            end_unit,
            weeks: activity
                .get("weekIndexes")
                .and_then(Value::as_array)
                .map(|weeks| {
                    weeks
                        .iter()
                        .filter_map(Value::as_i64)
                        .map(|w| w as u16)
                        .collect()
                })
                .unwrap_or_default(),
        });
    }
    Ok(Timetable {
        courses,
        semester_start_date: None,
    })
}

/// Parse the postgraduate (yjsxk) `loadKbxx` payload.
///
/// Shape: `results[]` with `KCMC` (course), `JASMC` (room), `JSXM` (teacher),
/// `ZCBH` (week mask, one 0/1 char per week), `XQ` (weekday 1-7), `KSJCDM`
/// (start period). Each record covers one period.
pub fn parse_postgraduate(payload: &Value) -> Result<Timetable, AppError> {
    let results = payload
        .get("results")
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::Upstream("课表数据格式无法解析".to_owned()))?;

    let mut courses = Vec::new();
    for record in results {
        let weekday = record.get("XQ").and_then(Value::as_i64).unwrap_or(1) as u8;
        let start_unit = record.get("KSJCDM").and_then(Value::as_i64).unwrap_or(1) as u8;
        // The upstream sends one record per period; merge consecutive periods
        // of the same course into one slot for display.
        let weeks = record
            .get("ZCBH")
            .and_then(Value::as_str)
            .map(weeks_from_mask)
            .unwrap_or_default();
        let course = TimetableCourse {
            course_name: record
                .get("KCMC")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            room_name: record
                .get("JASMC")
                .and_then(Value::as_str)
                .map(str::to_owned),
            teacher_names: record
                .get("JSXM")
                .and_then(Value::as_str)
                .map(|teacher| vec![teacher.to_owned()])
                .unwrap_or_default(),
            weekday,
            start_unit,
            end_unit: start_unit,
            weeks,
        };
        merge_into(&mut courses, course);
    }
    Ok(Timetable {
        courses,
        semester_start_date: None,
    })
}

/// Merge a single-period slot into the previous one when they continue the
/// same course on the same day with overlapping weeks.
fn merge_into(courses: &mut Vec<TimetableCourse>, course: TimetableCourse) {
    if let Some(last) = courses.last_mut() {
        if last.course_name == course.course_name
            && last.weekday == course.weekday
            && course.start_unit == last.end_unit + 1
            && last.room_name == course.room_name
            && last.weeks.iter().any(|week| course.weeks.contains(week))
        {
            last.end_unit = course.end_unit;
            return;
        }
    }
    courses.push(course);
}

/// `ZCBH` is one character per week ('1' = has class). The Flutter client
/// prefixes a zero so index == week number.
fn weeks_from_mask(mask: &str) -> Vec<u16> {
    let prefixed = format!("0{mask}");
    prefixed
        .char_indices()
        .filter(|(_, c)| *c == '1')
        .map(|(index, _)| index as u16)
        .collect()
}

/// Extract the semester list from the JWGL course-table page.
///
/// The page embeds `var semesters = JSON.parse('…');`; entries carry `id` and
/// a Sunday `startDate` that we shift to Monday, matching the Flutter client.
pub fn parse_semester_start(html: &str) -> Option<String> {
    let start = html.find("var semesters = JSON.parse(")? + "var semesters = JSON.parse(".len();
    let rest = &html[start..];
    let end = rest.find(");")?;
    let raw = rest[..end].trim().trim_matches('\'').replace("\\\"", "\"");
    let semesters: Value = serde_json::from_str(&raw).ok()?;
    let entries = semesters.as_array()?;
    // Entries come newest-first; pick the first with a usable start date.
    for entry in entries {
        let Some(date) = entry.get("startDate").and_then(Value::as_str) else {
            continue;
        };
        // `2026-09-06` (a Sunday) → `2026-09-07` (Monday).
        if let Ok(sunday) = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d") {
            return sunday
                .succ_opt()
                .map(|monday| monday.format("%Y-%m-%d").to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_jwgl_activities() {
        let payload = json!({
            "studentTableVms": [{
                "activities": [{
                    "courseName": "计算机系统基础",
                    "room": "HGX204",
                    "teachers": ["张三"],
                    "weekIndexes": [1, 2, 3],
                    "weekday": 3,
                    "startUnit": 1,
                    "endUnit": 2
                }]
            }]
        });
        let timetable = parse_jwgl(&payload).unwrap();
        assert_eq!(timetable.courses.len(), 1);
        assert_eq!(timetable.courses[0].course_name, "计算机系统基础");
        assert_eq!(timetable.courses[0].weekday, 3);
        assert_eq!(timetable.courses[0].weeks, vec![1, 2, 3]);
    }

    #[test]
    fn merges_postgraduate_periods() {
        let payload = json!({
            "results": [
                { "KCMC": "自然语言处理", "JASMC": "H3209", "JSXM": "李四",
                  "ZCBH": "1100000000000000000", "XQ": 2, "KSJCDM": 1 },
                { "KCMC": "自然语言处理", "JASMC": "H3209", "JSXM": "李四",
                  "ZCBH": "1100000000000000000", "XQ": 2, "KSJCDM": 2 },
                { "KCMC": "自然语言处理", "JASMC": "H3209", "JSXM": "李四",
                  "ZCBH": "1100000000000000000", "XQ": 2, "KSJCDM": 3 }
            ]
        });
        let timetable = parse_postgraduate(&payload).unwrap();
        assert_eq!(timetable.courses.len(), 1);
        assert_eq!(timetable.courses[0].start_unit, 1);
        assert_eq!(timetable.courses[0].end_unit, 3);
        assert_eq!(timetable.courses[0].weeks, vec![1, 2]);
    }

    #[test]
    fn parses_semester_start_from_html() {
        let html = "var semesters = JSON.parse('[{\"id\":123,\"startDate\":\"2026-09-06\"}]');";
        assert_eq!(parse_semester_start(html).as_deref(), Some("2026-09-07"));
    }
}
