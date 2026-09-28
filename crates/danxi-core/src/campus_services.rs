use std::time::Duration;

use regex::Regex;
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use serde_json::Value;

use crate::{
    AppError, CampusBus, CampusSession, DiningCrowdedness, DiningVenueOccupancy, EmptyClassroom,
    LibraryOccupancy,
};

const LIBRARY_URL: &str = "https://mlibrary.fudan.edu.cn/api/common/h5/getspaceseat";
const DINING_URL: &str = "https://my.fudan.edu.cn/simple_list/stqk";
const BUS_URL: &str = "https://zlapp.fudan.edu.cn/fudanbus/wap/default/lists";
const CLASSROOM_HOST: &str = "10.64.130.6";

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CampusLocation {
    Handan,
    Fenglin,
    Jiangwan,
    Zhangjiang,
}

impl CampusLocation {
    fn index(self) -> usize {
        match self {
            Self::Handan => 0,
            Self::Fenglin => 1,
            Self::Jiangwan => 2,
            Self::Zhangjiang => 3,
        }
    }
}

pub fn teaching_buildings(campus: CampusLocation) -> &'static [&'static str] {
    match campus {
        CampusLocation::Handan => &["HGD", "HGX", "H2", "H3", "H4", "H5", "H6"],
        CampusLocation::Fenglin => &["F1", "F2"],
        CampusLocation::Jiangwan => &["JA", "JB"],
        CampusLocation::Zhangjiang => &["Z2"],
    }
}

pub struct CampusLifeService {
    http: Client,
}

impl CampusLifeService {
    pub fn new() -> Result<Self, AppError> {
        let mut builder = Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(12))
            .user_agent(format!("DanXi Next/{}", env!("CARGO_PKG_VERSION")));
        if let Ok(proxy_url) = std::env::var("DANXI_HTTP_PROXY") {
            if !proxy_url.trim().is_empty() {
                builder = builder.proxy(reqwest::Proxy::all(&proxy_url).map_err(|_| {
                    AppError::Configuration("DANXI_HTTP_PROXY 配置无效".to_owned())
                })?);
            }
        }
        Ok(Self {
            http: builder
                .build()
                .map_err(|_| AppError::Configuration("HTTP 客户端初始化失败".to_owned()))?,
        })
    }

    pub async fn library_occupancy(&self) -> Result<Vec<LibraryOccupancy>, AppError> {
        let response = self.http.post(LIBRARY_URL).send().await?;
        ensure_success(response.status())?;
        let body: RawLibraryResponse = response
            .json()
            .await
            .map_err(|_| AppError::Upstream("图书馆人数响应格式无法解析".to_owned()))?;
        Ok(body
            .data
            .into_iter()
            .filter_map(|item| {
                Some(LibraryOccupancy {
                    campus_name: item.campus_name?.trim().to_owned(),
                    people: integer_value(item.in_num?)?,
                })
            })
            .collect())
    }

    pub async fn dining_crowdedness(
        &self,
        campus: &CampusSession,
        location: CampusLocation,
    ) -> Result<DiningCrowdedness, AppError> {
        let request = self.http.get(DINING_URL).build()?;
        let response = campus.authenticated_request(request).await?;
        ensure_success(response.status())?;
        let html = response
            .text()
            .await
            .map_err(|_| AppError::Upstream("食堂拥挤度页面读取失败".to_owned()))?;
        parse_dining(&html, location.index())
    }

    pub async fn bus_schedule(
        &self,
        campus: &CampusSession,
        holiday: bool,
    ) -> Result<Vec<CampusBus>, AppError> {
        let request = self
            .http
            .post(BUS_URL)
            .form(&[("holiday", if holiday { "1" } else { "0" })])
            .build()?;
        let response = campus.authenticated_request(request).await?;
        ensure_success(response.status())?;
        let body: Value = response
            .json()
            .await
            .map_err(|_| AppError::Upstream("校车时刻响应格式无法解析".to_owned()))?;
        parse_buses(&body)
    }

    pub async fn empty_classrooms(
        &self,
        campus: &CampusSession,
        building: &str,
        date: &str,
        use_webvpn: bool,
    ) -> Result<Vec<EmptyClassroom>, AppError> {
        let building = building.trim().to_ascii_uppercase();
        if !teaching_buildings(CampusLocation::Handan).contains(&building.as_str())
            && !teaching_buildings(CampusLocation::Fenglin).contains(&building.as_str())
            && !teaching_buildings(CampusLocation::Jiangwan).contains(&building.as_str())
            && !teaching_buildings(CampusLocation::Zhangjiang).contains(&building.as_str())
        {
            return Err(AppError::Configuration("不支持的教学楼".to_owned()));
        }
        if !valid_iso_date(date) {
            return Err(AppError::Configuration(
                "日期格式应为 YYYY-MM-DD".to_owned(),
            ));
        }
        let encoded_building = urlencoding::encode(&building);
        let encoded_date = urlencoding::encode(date);
        let status_url = format!(
            "http://{CLASSROOM_HOST}/daystatus.asp?b={encoded_building}&day={encoded_date}"
        );
        let detail_url =
            format!("http://{CLASSROOM_HOST}/?b={encoded_building}&c=&p=&day={encoded_date}");
        let status_request = self
            .http
            .get(status_url)
            .timeout(Duration::from_secs(4))
            .build()?;
        let detail_request = self
            .http
            .get(detail_url)
            .timeout(Duration::from_secs(4))
            .build()?;
        let (status_response, detail_response) = campus
            .routed_request_pair(status_request, detail_request, use_webvpn)
            .await?;
        ensure_success(status_response.status())?;
        let status_body = status_response
            .text()
            .await
            .map_err(|_| AppError::Upstream("教室状态读取失败".to_owned()))?;
        ensure_success(detail_response.status())?;
        let detail_body = detail_response
            .text()
            .await
            .map_err(|_| AppError::Upstream("教室课表读取失败".to_owned()))?;
        parse_classrooms(&status_body, &detail_body)
    }
}

fn ensure_success(status: StatusCode) -> Result<(), AppError> {
    if status.is_success() {
        Ok(())
    } else if status == StatusCode::UNAUTHORIZED {
        Err(AppError::Auth("校园服务登录状态已过期".to_owned()))
    } else {
        Err(AppError::upstream_status(status))
    }
}

#[derive(Deserialize)]
struct RawLibraryResponse {
    #[serde(default)]
    data: Vec<RawLibraryItem>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawLibraryItem {
    campus_name: Option<String>,
    in_num: Option<Value>,
}

fn integer_value(value: Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_str()?.trim().parse::<i64>().ok())
}

fn parse_dining(html: &str, campus_index: usize) -> Result<DiningCrowdedness, AppError> {
    if html.contains('仅') {
        return Ok(DiningCrowdedness {
            available: false,
            venues: Vec::new(),
        });
    }
    let script_start = html
        .find("<script>")
        .map(|index| index + "<script>".len())
        .ok_or_else(|| AppError::Upstream("食堂拥挤度页面格式无法解析".to_owned()))?;
    let script_end = html[script_start..]
        .find("</script>")
        .map(|index| script_start + index)
        .ok_or_else(|| AppError::Upstream("食堂拥挤度页面格式无法解析".to_owned()))?;
    let script = html[script_start..script_end].replace(r"\n", "-");
    let arrays = Regex::new(r"\[[^\]]*\]")
        .expect("static dining regex")
        .find_iter(&script)
        .map(|item| item.as_str().replace('\'', "\""))
        .collect::<Vec<_>>();
    let offset = campus_index * 3;
    let names: Vec<String> = parse_array(arrays.get(offset))?;
    let current: Vec<String> = parse_array(arrays.get(offset + 1))?;
    let capacity: Vec<String> = parse_array(arrays.get(offset + 2))?;
    let venues = names
        .into_iter()
        .enumerate()
        .filter_map(|(index, name)| {
            Some(DiningVenueOccupancy {
                name,
                current: current.get(index)?.parse().ok()?,
                capacity: capacity.get(index)?.parse().ok()?,
            })
        })
        .collect();
    Ok(DiningCrowdedness {
        available: true,
        venues,
    })
}

fn parse_array<T>(value: Option<&String>) -> Result<Vec<T>, AppError>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_str(
        value.ok_or_else(|| AppError::Upstream("食堂拥挤度数据不完整".to_owned()))?,
    )
    .map_err(|_| AppError::Upstream("食堂拥挤度数据无法解析".to_owned()))
}

fn parse_buses(body: &Value) -> Result<Vec<CampusBus>, AppError> {
    let routes = body
        .get("d")
        .and_then(|value| value.get("data"))
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::Upstream("校车时刻响应缺少数据".to_owned()))?;
    let mut buses = Vec::new();
    for route in routes {
        let Some(items) = route.get("lists").and_then(Value::as_array) else {
            continue;
        };
        for item in items {
            let start_time = string_value(item.get("stime")).and_then(normalize_time);
            let end_time = string_value(item.get("etime")).and_then(normalize_time);
            if start_time.is_none() && end_time.is_none() {
                continue;
            }
            buses.push(CampusBus {
                id: string_value(item.get("id")).unwrap_or_default(),
                start_campus: normalize_campus(
                    &string_value(item.get("start")).unwrap_or_default(),
                ),
                end_campus: normalize_campus(&string_value(item.get("end")).unwrap_or_default()),
                start_time,
                end_time,
                direction: integer_value(item.get("arrow").cloned().unwrap_or(Value::Null))
                    .unwrap_or_default(),
                holiday_run: integer_value(item.get("holiday").cloned().unwrap_or(Value::Null))
                    .unwrap_or_default()
                    != 0,
            });
        }
    }
    buses.sort_by(|left, right| {
        left.start_time
            .as_deref()
            .or(left.end_time.as_deref())
            .cmp(&right.start_time.as_deref().or(right.end_time.as_deref()))
    });
    Ok(buses)
}

fn string_value(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

fn normalize_time(value: String) -> Option<String> {
    let value = value.trim().replace('.', ":");
    (!value.is_empty()).then_some(value)
}

fn normalize_campus(value: &str) -> String {
    ["邯郸", "枫林", "江湾", "张江"]
        .into_iter()
        .find(|campus| value.contains(campus))
        .unwrap_or(value.trim())
        .to_owned()
}

fn parse_classrooms(status_body: &str, detail_body: &str) -> Result<Vec<EmptyClassroom>, AppError> {
    let status_match = Regex::new(r#"(?s)"status"\s*:\s*(\[.*?\])\s*,\s*"tempnotice""#)
        .expect("static classroom regex")
        .captures(status_body)
        .and_then(|captures| captures.get(1))
        .ok_or_else(|| AppError::Upstream("教室状态格式无法解析".to_owned()))?;
    let rooms: Vec<Value> = serde_json::from_str(status_match.as_str())
        .map_err(|_| AppError::Upstream("教室状态格式无法解析".to_owned()))?;
    let seats_regex = Regex::new(r">(\d+)<").expect("static seats regex");
    let usage_regex =
        Regex::new(r#"<td style="background-color.*?>(.*?)</td>"#).expect("static usage regex");
    let mut result = Vec::new();
    for (index, room) in rooms.iter().enumerate() {
        let room_name = string_value(room.get("room")).unwrap_or_default();
        let room_id = string_value(room.get("id")).unwrap_or_default();
        if room_name.is_empty() || room_id.is_empty() {
            continue;
        }
        let start_marker = format!("\"c{room_id}\"");
        let Some(start) = detail_body.find(&start_marker) else {
            continue;
        };
        let end = rooms
            .get(index + 1)
            .and_then(|next| string_value(next.get("id")))
            .and_then(|next_id| detail_body[start..].find(&format!("\"r{next_id}\"")))
            .map(|relative| start + relative)
            .or_else(|| {
                detail_body[start..]
                    .find("innerHTML")
                    .map(|relative| start + relative)
            })
            .unwrap_or(detail_body.len());
        let html = &detail_body[start..end];
        let seats = seats_regex
            .captures(html)
            .and_then(|captures| captures.get(1))
            .and_then(|value| value.as_str().parse().ok());
        let mut busy = usage_regex
            .captures_iter(html)
            .filter_map(|captures| captures.get(1))
            .map(|value| !value.as_str().trim().is_empty())
            .collect::<Vec<_>>();
        if busy.len() > 13 {
            busy.truncate(13);
        }
        result.push(EmptyClassroom {
            room_name,
            seats,
            busy,
        });
    }
    Ok(result)
}

fn valid_iso_date(value: &str) -> bool {
    chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_library_payload() {
        let raw: RawLibraryResponse = serde_json::from_value(serde_json::json!({
            "data": [{"campusName": "邯郸校区", "inNum": "1234"}]
        }))
        .unwrap();
        let item = raw.data.into_iter().next().unwrap();
        assert_eq!(item.campus_name.as_deref(), Some("邯郸校区"));
        assert_eq!(integer_value(item.in_num.unwrap()), Some(1234));
    }

    #[test]
    fn parses_dining_arrays_for_selected_campus() {
        let html = "<script>var a=['北区-一楼'];var b=['100'];var c=['200'];var d=['枫林食堂'];var e=['50'];var f=['120'];</script>";
        let result = parse_dining(html, 1).unwrap();
        assert!(result.available);
        assert_eq!(result.venues[0].name, "枫林食堂");
        assert_eq!(result.venues[0].current, 50);
    }

    #[test]
    fn parses_bus_payload() {
        let body = serde_json::json!({"d":{"data":[{"lists":[{
            "id":"1","start":"邯郸校区","end":"江湾校区","stime":"08.30",
            "etime":"09:10","arrow":"1","holiday":"0"
        }]}]}});
        let buses = parse_buses(&body).unwrap();
        assert_eq!(buses[0].start_campus, "邯郸");
        assert_eq!(buses[0].start_time.as_deref(), Some("08:30"));
    }

    #[test]
    fn parses_classroom_slots() {
        let status = r#"{"status":[{"id":"586","room":"HGX103"}],"tempnotice":""}"#;
        let detail = r#""c586"><b>80</b><td style="background-color:x"></td><td style="background-color:x">课程</td>innerHTML"#;
        let rooms = parse_classrooms(status, detail).unwrap();
        assert_eq!(rooms[0].room_name, "HGX103");
        assert_eq!(rooms[0].seats, Some(80));
        assert_eq!(rooms[0].busy, vec![false, true]);
    }
}
