use serde::{Deserialize, Serialize};

/// Specifies the optional format for the path shape of each connection
#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeFormat {
    #[serde(rename = "polyline6")]
    Polyline6,
    #[serde(rename = "polyline5")]
    Polyline5,
    #[serde(rename = "geojson")]
    GeoJSON,
    #[serde(rename = "no_shape")]
    NoShape,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ShapePoint {
    pub lon: f64,
    pub lat: f64,
}

impl From<&ShapePoint> for geo_types::Point {
    fn from(p: &ShapePoint) -> Self {
        debug_assert!((-90.0..90.0).contains(&p.lat));
        debug_assert!((-180.0..180.0).contains(&p.lon));
        Self::new(p.lon, p.lat)
    }
}

impl From<ShapePoint> for super::Coordinate {
    fn from(p: ShapePoint) -> Self {
        debug_assert!((-90.0..90.0).contains(&p.lat));
        debug_assert!((-180.0..180.0).contains(&p.lon));
        (p.lon as f32, p.lat as f32)
    }
}

/// decodes polyline6 to [`ShapePoint`]s
///
/// Algorithm based on https://valhalla.github.io/valhalla/decoding/#python
fn decode_shape_polyline6(encoded: &str) -> Vec<ShapePoint> {
    debug_assert!(encoded.is_ascii());
    debug_assert!(!encoded.is_empty());
    // six degrees of precision in valhalla
    let inv = 1.0 / 1e6;
    let mut decoded = Vec::new();
    let mut previous = [0, 0];
    let mut i = 0;

    while i < encoded.len() {
        // for each coord (lat, lon)
        let mut ll = [0, 0];
        for j in [0, 1] {
            let mut shift = 0;
            let mut byte = 0x20;
            // keep decoding bytes until you have this coord
            while byte >= 0x20 {
                byte = i32::from(encoded.as_bytes()[i]) - 63;
                i += 1;
                ll[j] |= (byte & 0x1f) << shift;
                shift += 5;
            }
            // get the final value adding the previous offset and remember it for the next
            ll[j] = previous[j]
                + if (ll[j] & 1) != 0 {
                !(ll[j] >> 1)
            } else {
                ll[j] >> 1
            };
            previous[j] = ll[j];
        }
        // scale by the precision
        let lon = f64::from(ll[1]) * inv;
        let lat = f64::from(ll[0]) * inv;
        debug_assert!((-90.0..90.0).contains(&lat));
        debug_assert!((-180.0..180.0).contains(&lon));
        decoded.push(ShapePoint { lon, lat });
    }

    decoded
}
pub(crate) fn deserialize_shape<'de, D>(deserializer: D) -> Result<Vec<ShapePoint>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(decode_shape_polyline6(s.as_str()))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decode_shape_works_america() {
        // shape from https://valhalla1.openstreetmap.de/optimized_route?json=%7B%22locations%22%3A%5B%7B%22lat%22%3A40.042072%2C%22lon%22%3A-76.306572%7D%2C%7B%22lat%22%3A39.991889%2C%22lon%22%3A-76.781939%7D%2C%7B%22lat%22%3A39.984460%2C%22lon%22%3A-76.695075%7D%2C%7B%22lat%22%3A39.996900%2C%22lon%22%3A-76.768704%7D%2C%7B%22lat%22%3A39.983901%2C%22lon%22%3A-76.707604%7D%5D%2C%22costing%22%3A%22auto%22%2C%22units%22%3A%22kilometers%22%7D
        let shape = decode_shape_polyline6("}c|gkAlvkmqCkg@zf@_IbJaP`XoXq^aCwDyByGkAyGI}H?iFZeH`AuFfB}HxBaG~xAiaBtFgHtPqWfCeDpMsNva@px@rRp_@|s@vvAd{@tbBpiAfzBjNuw@lGo`@bAwHz@}I|@cQ^gNf@_OB_NJ}[BeJH{c@X{[SaK_@{S]}Q_@cJUgLAgJ?iUBoB^eMFoA`BsXbLgxAzK_{CpD}oA`@wUlBegAvAmh@fGcr@lGur@zGu]jGeSv@_BdNiYdCaDhHaJx^_[f^}Qre@cXjOgJtJ}HjIqKjEkHzD{HpDaIrB_GvA}EpAoFfAaGlA}H|@yHbAwK~Le{A|@yHv@{Ft@cEfBiHdCsHjCkF|ByDxQaYlE}GrCmEjR{Yp^al@nMgM|C_Fb]_j@xLwR~Ro[`DiElDsD~GmGlNmMpEwDkAwGcDwR_EkUmDuV{CmTiPwaAsc@{kCwL_t@{d@wmCqZkiB{NkcAcGwa@aAgHyJ_t@uI_q@_Kyp@aEgYqBqL}M_v@_Q{n@sVw}@gV{r@kLs\\sCeIqXmw@eFk]W{]dCic@Dw@vIsb@p@gD~Oiw@hAkGtBaLd@gFjAwc@GiGOeKs@ce@i@_HeEci@_@eFyDsh@gEsr@a@eZqAuaAo@cnAb@}JhAwpAnCq|CpCocBHcDZiNrAcp@`Biz@x@}m@bAgl@x@cWrHycAbGyi@tNe~@rAsIjDqTlDwS|G_g@vGyd@fGes@~DynAZ}nAXc~BoGeB}HsIaL}CqMeDmDbBwBe@iF}@wMkCkSuFsA_@");
        // generated via http://valhalla.github.io/demos/polyline/
        let expected = [
            [-76.781943, 39.991887],
            [-76.782581, 39.992533],
            [-76.782759, 39.992692999999996],
            [-76.78316, 39.992965999999996],
            [-76.78265499999999, 39.993373999999996],
            [-76.782563, 39.993438999999995],
            [-76.782422, 39.9935],
            [-76.782281, 39.993538],
            [-76.782122, 39.993542999999995],
            [-76.782005, 39.993542999999995],
            [-76.781858, 39.993528999999995],
            [-76.781735, 39.993496],
            [-76.781576, 39.993444],
            [-76.781447, 39.993383],
            [-76.77987399999999, 39.991943],
            [-76.779726, 39.99182],
            [-76.779333, 39.991537],
            [-76.77924999999999, 39.991468999999995],
            [-76.779, 39.991236],
            [-76.779921, 39.99068],
            [-76.780442, 39.990366],
            [-76.781846, 39.989519],
            [-76.783441, 39.988555999999996],
            [-76.78541299999999, 39.987362999999995],
            [-76.784506, 39.987117],
            [-76.78397, 39.986982],
            [-76.78381399999999, 39.986948],
            [-76.783639, 39.986917999999996],
            [-76.783349, 39.986886999999996],
            [-76.78310499999999, 39.986871],
            [-76.782849, 39.986851],
            [-76.782609, 39.986849],
            [-76.782146, 39.986843],
            [-76.781967, 39.986841],
            [-76.78137699999999, 39.986836],
            [-76.780915, 39.986823],
            [-76.780722, 39.986833],
            [-76.780388, 39.986849],
            [-76.780085, 39.986864],
            [-76.779907, 39.98688],
            [-76.77969499999999, 39.986891],
            [-76.779515, 39.986892],
            [-76.779158, 39.986892],
            [-76.779102, 39.986889999999995],
            [-76.778875, 39.986874],
            [-76.778835, 39.986869999999996],
            [-76.778425, 39.986821],
            [-76.776997, 39.986610999999996],
            [-76.774501, 39.986405],
            [-76.773206, 39.986315999999995],
            [-76.772842, 39.986298999999995],
            [-76.771687, 39.986244],
            [-76.771024, 39.9862],
            [-76.770206, 39.986067999999996],
            [-76.769379, 39.985932999999996],
            [-76.76888799999999, 39.985791],
            [-76.768565, 39.985656999999996],
            [-76.768517, 39.985628999999996],
            [-76.768096, 39.985386],
            [-76.76801499999999, 39.985319],
            [-76.767838, 39.98517],
            [-76.76738999999999, 39.984660999999996],
            [-76.767087, 39.984161],
            [-76.766685, 39.983543],
            [-76.766505, 39.983281],
            [-76.766346, 39.983094],
            [-76.766145, 39.982928],
            [-76.76599499999999, 39.982825999999996],
            [-76.76583699999999, 39.982732],
            [-76.765676, 39.982642999999996],
            [-76.765548, 39.982585],
            [-76.76543699999999, 39.982541],
            [-76.765317, 39.9825],
            [-76.765188, 39.982464],
            [-76.765029, 39.982425],
            [-76.764872, 39.982394],
            [-76.764668, 39.98236],
            [-76.763193, 39.982136],
            [-76.763036, 39.982105],
            [-76.76290999999999, 39.982077],
            [-76.762812, 39.98205],
            [-76.762663, 39.981998],
            [-76.762509, 39.981930999999996],
            [-76.762391, 39.981860999999995],
            [-76.762298, 39.981798],
            [-76.761881, 39.981497],
            [-76.761738, 39.981394],
            [-76.761635, 39.98132],
            [-76.76120499999999, 39.98101],
            [-76.76048399999999, 39.980505],
            [-76.760256, 39.980273],
            [-76.760144, 39.980194],
            [-76.759456, 39.979712],
            [-76.75914, 39.979490999999996],
            [-76.758684, 39.979171],
            [-76.758583, 39.97909],
            [-76.758493, 39.979003],
            [-76.758358, 39.978859],
            [-76.758127, 39.978612],
            [-76.75803499999999, 39.978507],
            [-76.75789499999999, 39.978545],
            [-76.75757899999999, 39.978626999999996],
            [-76.757221, 39.978722999999995],
            [-76.75684199999999, 39.978809999999996],
            [-76.75649899999999, 39.978888],
            [-76.755431, 39.979164999999995],
            [-76.753177, 39.979751],
            [-76.752329, 39.979971],
            [-76.750045, 39.980577],
            [-76.74834299999999, 39.981018],
            [-76.747249, 39.981272],
            [-76.746693, 39.981401999999996],
            [-76.746545, 39.981435],
            [-76.74569699999999, 39.981624],
            [-76.744897, 39.981795],
            [-76.7441, 39.981987],
            [-76.74368, 39.982084],
            [-76.74346299999999, 39.982141],
            [-76.742583, 39.98238],
            [-76.741817, 39.982668],
            [-76.740813, 39.983046],
            [-76.739983, 39.983418],
            [-76.739509, 39.983632],
            [-76.739346, 39.983706],
            [-76.73844299999999, 39.984114999999996],
            [-76.737957, 39.98423],
            [-76.73746299999999, 39.984241999999995],
            [-76.736882, 39.984175],
            [-76.736854, 39.984172],
            [-76.736284, 39.983999999999995],
            [-76.7362, 39.983975],
            [-76.735299, 39.983703],
            [-76.735165, 39.983666],
            [-76.734956, 39.983607],
            [-76.73483999999999, 39.983588],
            [-76.734252, 39.98355],
            [-76.73411899999999, 39.983554],
            [-76.733924, 39.983562],
            [-76.733314, 39.983588],
            [-76.73317, 39.983609],
            [-76.732496, 39.983708],
            [-76.73238099999999, 39.983723999999995],
            [-76.731715, 39.983816999999995],
            [-76.73088899999999, 39.983917],
            [-76.730454, 39.983934],
            [-76.729387, 39.983975],
            [-76.728121, 39.983999],
            [-76.72793, 39.983981],
            [-76.72662199999999, 39.983944],
            [-76.72410099999999, 39.983872],
            [-76.722493, 39.983799],
            [-76.722411, 39.983793999999996],
            [-76.722166, 39.983779999999996],
            [-76.72138, 39.983737999999995],
            [-76.72043099999999, 39.983689],
            [-76.71968, 39.98366],
            [-76.71895599999999, 39.983626],
            [-76.71857, 39.983596999999996],
            [-76.717469, 39.983443],
            [-76.71678399999999, 39.983312999999995],
            [-76.715773, 39.983062],
            [-76.715603, 39.983019999999996],
            [-76.71525799999999, 39.982934],
            [-76.71492599999999, 39.982847],
            [-76.714286, 39.982704],
            [-76.713681, 39.982563999999996],
            [-76.712846, 39.982431999999996],
            [-76.711569, 39.982336],
            [-76.71029, 39.982321999999996],
            [-76.70825599999999, 39.982309],
            [-76.70820499999999, 39.982445],
            [-76.708035, 39.982603999999995],
            [-76.707956, 39.982813],
            [-76.70787299999999, 39.983046],
            [-76.707923, 39.983132999999995],
            [-76.707904, 39.983193],
            [-76.70787299999999, 39.983309999999996],
            [-76.707803, 39.983546],
            [-76.70768, 39.983872],
            [-76.707664, 39.983914],
        ];
        let expected = expected
            .into_iter()
            .map(|[lon, lat]| ShapePoint { lat, lon })
            .collect::<Vec<_>>();
        assert_eq!(shape, expected);
    }
    #[test]
    fn decode_shape_works_germany() {
        let shape = decode_shape_polyline6("czaa{AythgU}K_CgFeAiB]mDq@uRoD_Ca@|@aOb@uHd@eIb@gHh@wI`@cHNmChBa[|Cih@fA_RzB^fm@fK~AVbLlBpHnAvMfCvDt@hMzBrOjCtGfArEz@dJvAdC^bC@jH~B^bBXjARvZnFzV|EpNjCrRnDpS~D`Dd@bK`BjEp@lCd@jLxBlI~A~F|QT`Ag@~Ga@pEYrCa@fExA`@~IfCzIjCj{@|Up}@hWlTpHpAbB^`C}Czh@}FgBmCy@sOqEwEjb@o@rFoAdLeAa@yIaDcFiBYdC");
        // generated via http://valhalla.github.io/demos/polyline/
        let expected = [
            [11.670365, 48.268722],
            [11.670428999999999, 48.268929],
            [11.670463999999999, 48.269045],
            [11.670479, 48.269098],
            [11.670504, 48.269185],
            [11.670592, 48.2695],
            [11.670608999999999, 48.269563999999995],
            [11.670866, 48.269532999999996],
            [11.671021, 48.269515],
            [11.671184, 48.269496],
            [11.671332, 48.269478],
            [11.671503999999999, 48.269456999999996],
            [11.67165, 48.269439999999996],
            [11.671721, 48.269431999999995],
            [11.67217, 48.269379],
            [11.672830999999999, 48.2693],
            [11.673135, 48.269264],
            [11.673119, 48.269202],
            [11.672922999999999, 48.268462],
            [11.672911, 48.268414],
            [11.672856, 48.268204],
            [11.672816, 48.268051],
            [11.672748, 48.267815],
            [11.672721, 48.267723],
            [11.672659, 48.267494],
            [11.672589, 48.267227999999996],
            [11.672552999999999, 48.267089],
            [11.672523, 48.266982999999996],
            [11.672479, 48.266804],
            [11.672462999999999, 48.266737],
            [11.672467, 48.266670999999995],
            [11.672317, 48.26667],
            [11.672301, 48.266605999999996],
            [11.672288, 48.266556],
            [11.672277999999999, 48.266518],
            [11.672158, 48.266073999999996],
            [11.672047, 48.265691999999994],
            [11.671977, 48.265443],
            [11.671889, 48.265128999999995],
            [11.671793, 48.2648],
            [11.671774, 48.264719],
            [11.671725, 48.264525],
            [11.6717, 48.264423],
            [11.671681, 48.264351999999995],
            [11.671619999999999, 48.264137999999996],
            [11.671572, 48.263971],
            [11.671268999999999, 48.263842999999994],
            [11.671235999999999, 48.263832],
            [11.671092, 48.263852],
            [11.670987, 48.263869],
            [11.670912999999999, 48.263881999999995],
            [11.670812999999999, 48.263898999999995],
            [11.670796, 48.263853999999995],
            [11.670727999999999, 48.263678],
            [11.670658, 48.263504],
            [11.670290999999999, 48.262538],
            [11.669901999999999, 48.261537],
            [11.669749, 48.261193999999996],
            [11.669699, 48.261153],
            [11.669634, 48.261137],
            [11.668963999999999, 48.261216],
            [11.669016, 48.261343],
            [11.669044999999999, 48.261413999999995],
            [11.66915, 48.26168],
            [11.668584, 48.261787999999996],
            [11.668462, 48.261812],
            [11.668251, 48.261852],
            [11.668268, 48.261886999999994],
            [11.668349, 48.26206],
            [11.668401999999999, 48.262173999999995],
            [11.668334999999999, 48.262187],
        ];
        let expected = expected
            .into_iter()
            .map(|[lon, lat]| ShapePoint { lat, lon })
            .collect::<Vec<_>>();
        assert_eq!(shape, expected);
    }
}
