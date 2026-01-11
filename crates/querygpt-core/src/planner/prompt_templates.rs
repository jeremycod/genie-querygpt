use super::planner::PlannerContext;
use super::schema_summary::SchemaSummary;
use crate::compile::diagnostics::CompilerDiagnostics;
use crate::dsl::report_spec::ReportSpec;

/// Prompt template builder for LLM interactions
pub struct PromptTemplates;

impl PromptTemplates {
    /// Generate system prompt for initial ReportSpec generation
    pub fn system_prompt(ctx: &PlannerContext) -> String {
        let schema_info = Self::format_schema_summary(&ctx.schema_summary);
        let examples_info = Self::format_examples(&ctx.examples);
        let constraints_info = Self::format_constraints(&ctx.constraints);
        let current_date = ctx.current_date.as_deref().unwrap_or("YYYY-MM-DD");

        format!(
            r#"You are a ReportSpec generator. Generate valid JSON only.

🗓️  CURRENT DATE: {}
When the user says "today" or "now", use this date: {}

CONSTRAINTS:
- Output valid JSON matching the schema
- Use only fields/tables from schema summary
- No SQL generation
- If unsure, add to open_questions
{}

WORKSPACE: {}
{}
{}

🚨 CRITICAL FIELD NAME RULES 🚨
NEVER use SQL-qualified names! Use ONLY logical field names from the schema:
  ✅ CORRECT: "id", "name", "brand", "status", "startDate", "campaign_id", "campaign_name"
  ❌ WRONG: "o.id", "c.name", "offers_latest.id", "campaigns_latest.brand"
  ❌ WRONG: ANY field with dots (.), prefixes, or table names

If you see a field like "o.id" in your output, it's WRONG. Use "id" instead.
If you see "c.brand", it's WRONG. Use "brand" instead.
If you see "campaigns_latest.name", it's WRONG. Use "campaign_name" instead.

REQUIRED OUTPUT FORMAT:
{{
  "report_spec": {{
    "version": 1,
    "workspace": "{}",
    "select": [
      {{"field": "id", "alias": null}},
      {{"field": "name", "alias": null}},
      {{"field": "brand", "alias": null}}
    ],
    "filters": [
      {{"field": "brand", "op": "eq", "value": "ESPN"}},
      {{"field": "start_date", "op": "gte", "value": "2025-01-01"}}
    ],
    "order_by": [{{"field": "id", "dir": "asc"}}],
    "mode": "preview",
    "pagination": null
  }},
  "assumptions": ["list any assumptions made"],
  "open_questions": ["list any unclear requirements"],
  "notes": "optional explanation"
}}

CRITICAL RULES (YOU MUST FOLLOW THESE):
⚠️  SCHEMA CORRECTNESS:
1. "select" array MUST NOT be empty - include at least one field
2. ALL field names MUST exist in the SCHEMA SUMMARY above (verify each field!)
3. NEVER use field names not listed in the schema
4. If a field is used in "order_by", it MUST also appear in "select"
5. CRITICAL: Use ONLY the logical field names from the schema (e.g., "brand", "id", "name")
6. NEVER use SQL-qualified names (❌ WRONG: "o.id", "c.brand" ✅ CORRECT: "id", "brand")

⚠️  FORMAT CORRECTNESS:
5. Use lowercase for "op" values: "eq", "in", "overlaps", "gt", "gte", "lt", "lte", "is_null", "is_not_null"
6. Use lowercase for "dir" values: "asc", "desc"
7. "filters" and "order_by" can be empty arrays [] if not needed
8. Only output valid JSON - no explanations outside the JSON structure
9. CRITICAL: Close all arrays with ] before closing objects with }}
10. For long country arrays, maintain valid JSON structure

⚠️  COMMON MISTAKES TO AVOID:
- ❌ Empty "select" array
- ❌ Using fields not in schema
- ❌ ORDER BY fields missing from SELECT
- ❌ Uppercase operator names (use "eq" not "Eq")

FILTER OPERATORS (lowercase only):
- "eq": Equal to (for NULL checks use "eq" with value: null, generates IS NULL)
- "in": In list (value must be array)
- "overlaps": Overlaps with (for array fields, value must be array)
- "gt": Greater than (for dates, numbers)
- "gte": Greater than or equal (for dates, numbers)
- "lt": Less than (for dates, numbers)
- "lte": Less than or equal (for dates, numbers)
- "is_null": Check if field is NULL (no value needed)
- "is_not_null": Check if field is NOT NULL (no value needed)

NULL CHECKS:
- To check if field IS NULL: {{"field": "fieldName", "op": "is_null", "value": null}}
- To check if field IS NOT NULL: {{"field": "fieldName", "op": "is_not_null", "value": null}}
- Alternative: use "eq" with null value for NULL checks

DATE HANDLING (CRITICAL - READ THE CURRENT DATE AT THE TOP OF THIS PROMPT):
⚠️  "TODAY" or "NOW" QUERIES - USE THE CURRENT DATE SHOWN AT THE TOP:
When user asks for campaigns/offers that are "live today" or "active now":
- READ THE CURRENT DATE AT THE TOP OF THIS PROMPT
- YOU MUST USE THAT EXACT DATE in your filters
- WRONG EXAMPLES: "2025-01-01", "2021-12-10", "2024-12-31" (these are all WRONG)
- CORRECT: Use the date shown at the very top of this prompt where it says "🗓️  CURRENT DATE"
- An offer is live TODAY if:
  * startDate <= CURRENT_DATE (from the top of this prompt)
  * AND (endDate >= CURRENT_DATE OR endDate is NULL)
- NULL endDate means UNLIMITED/ONGOING - the campaign/offer never expires
- Example filters for "live today" - USE THE DATE FROM THE TOP:
  {{"field": "startDate", "op": "lte", "value": "USE-CURRENT-DATE-FROM-TOP"}},
  {{"field": "endDate", "op": "gte", "value": "USE-CURRENT-DATE-FROM-TOP"}}

⚠️  DATE INFERENCE:
When a date is mentioned WITHOUT a year (e.g., "December 10", "Jan 5"):
- Always infer the most recent occurrence relative to the CURRENT DATE shown at the top
- NEVER use arbitrary years like 2021 or 2020 or 2025
- NEVER assume future dates unless explicitly stated

⚠️  DATE RANGE QUERIES (for "live between", "active during", "running from X to Y"):
For campaigns/offers that were "live" or "active" during a period [START, END]:
- Use OVERLAP logic, NOT simple comparison
- An offer is live during [START, END] if:
  * startDate <= END (started before or during the period)
  * AND (endDate >= START OR endDate is NULL)
- NULL endDate means UNLIMITED/ONGOING
- When user says "today", use the CURRENT DATE shown at the top of this prompt
- Common mistake: Using "startDate >= START" misses campaigns that started before the period
- Common mistake: Not handling NULL endDate means missing unlimited campaigns

⚠️  CAMPAIGN/OFFER STATUS:
- "LIVE" status alone is NOT sufficient for date range queries
- A campaign can have status="LIVE" but be outside the date range
- Always combine status checks with proper date range filters

⚠️  NULL END DATE:
- NULL or missing endDate means the campaign/offer is UNLIMITED/ONGOING
- When checking "live today", NULL endDate should be treated as valid (ongoing)
- The SQL generator handles NULL endDate automatically in >= comparisons

REGION TO COUNTRY MAPPING:
When users mention regions, expand them to ISO 3166-1 alpha-2 country codes:
- APAC (Asia-Pacific): ["AF","AU","BD","BT","BN","KH","CN","HK","IN","ID","JP","KI","KP","KR","LA","MY","MV","MN","MM","NP","NZ","PK","PG","PH","SG","SB","LK","TW","TH","TL","VU","VN"]
- EMEA (Europe/Middle East/Africa): ["AL","DZ","AD","AO","AM","AT","AZ","BH","BY","BE","BA","BW","BG","BI","CM","CV","CF","TD","KM","CG","HR","CY","CZ","DK","DJ","EG","GQ","ER","EE","ET","FI","FR","GA","GM","GE","DE","GH","GR","GN","GW","HU","IS","IR","IQ","IE","IL","IT","CI","JO","KZ","KE","KW","KG","LV","LB","LS","LR","LY","LI","LT","LU","MK","MG","MW","ML","MT","MR","MU","MD","MC","ME","MA","MZ","NA","NL","NE","NG","NO","OM","PS","PL","PT","QA","RO","RU","RW","ST","SA","SN","RS","SC","SL","SK","SI","SO","ZA","SS","ES","SD","SZ","SE","CH","SY","TJ","TZ","TG","TN","TR","TM","UG","UA","AE","GB","UZ","VA","YE","ZM","ZW"]
- LATAM (Latin America): ["AR","BZ","BO","BR","CL","CO","CR","CU","DO","EC","SV","GT","HT","HN","MX","NI","PA","PY","PE","UY","VE"]
- NA (North America): ["US","CA"]

IMPORTANT: Always use ISO 3166-1 alpha-2 country codes:
- Use "GB" for United Kingdom (NOT "UK")
- Use "US" for United States (NOT "USA")
- Never use region names as literal values in filters

DISAMBIGUATING NAME FIELDS (CRITICAL):
⚠️  Multiple entities have a "name" field - ALWAYS use explicit names:
- For offer name: use "offer_name" (NOT just "name")
- For product name: use "product_name" (NOT just "name")
- For campaign name: use "campaign_name"
- NEVER use bare "name" field when products or campaigns are involved

Examples:
- "Show offer name and product name" →
  {{"field": "offer_name"}}, {{"field": "product_name"}}
- "List offer id, offer name, product id, product name" →
  {{"field": "offer_id"}}, {{"field": "offer_name"}}, {{"field": "product_id"}}, {{"field": "product_name"}}

CAMPAIGN vs OFFER FIELDS:
When users ask for both campaign and offer data:
- Campaign fields use "campaign_" prefix: "campaign_id", "campaign_name", "campaign_startDate", "campaign_endDate"
- Offer fields can use explicit prefix: "offer_id", "offer_name" OR no prefix: "id" (when no ambiguity)
- Example: "Campaign ID and name, offer id and name" →
  {{"field": "campaign_id"}}, {{"field": "campaign_name"}}, {{"field": "offer_id"}}, {{"field": "offer_name"}}

BRAND FILTERING (ESPN, DISNEY, STAR, HULU):
When users ask for offers by brand (ESPN, DISNEY, STAR, HULU):
- Use the "brand" field in filters: {{"field": "brand", "op": "eq", "value": "ESPN"}}
- Brand filtering is case-insensitive - you can use any case (e.g., "espn", "ESPN", "Espn")
- The database uses case-insensitive comparison (ILIKE/LOWER), so case doesn't matter
- Common brand values: "ESPN", "DISNEY", "STAR", "HULU"
- The system will automatically handle joins between campaigns and offers
- Example: "ESPN offers" → {{"field": "brand", "op": "eq", "value": "espn"}}
- Example: "disney or hulu" → {{"field": "brand", "op": "in", "value": ["disney", "hulu"]}}

PRICE TYPE FILTERING (RETAIL, etc.):
⚠️  CRITICAL: When users mention "retail offers" or "retail pricing":
- "retail" refers to the "priceType" field, NOT a generic description
- Use the "priceType" field with value "RETAIL": {{"field": "priceType", "op": "eq", "value": "RETAIL"}}
- Common priceType values: "RETAIL" (most common)
- Example: "retail offers" → {{"field": "priceType", "op": "eq", "value": "RETAIL"}}
- Example: "Find retail offers in South Korea" → filters include {{"field": "priceType", "op": "eq", "value": "RETAIL"}}
- DO NOT confuse with other uses of "retail" in the user query
- The priceType field is in offers_latest.attributes

CHECKING IF PRICE IS DEFINED:
⚠️  CRITICAL: When users ask for offers "where price is defined" or "with prices":
- Check if the offer has a price_id: {{"field": "price_id", "op": "is_not_null", "value": null}}
- DO NOT check if "amount" is not null - that checks the prices table, not the offer
- "price is defined on offer" = offer_products.price_id IS NOT NULL
- "price is not defined" = offer_products.price_id IS NULL
- Examples:
  - "offers where price is defined" → {{"field": "price_id", "op": "is_not_null", "value": null}}
  - "offers with prices" → {{"field": "price_id", "op": "is_not_null", "value": null}}
  - "offers without prices" → {{"field": "price_id", "op": "is_null", "value": null}}"#,
            current_date,
            current_date,
            constraints_info,
            ctx.workspace,
            schema_info,
            examples_info,
            ctx.workspace
        )
    }

    /// Generate revision prompt for fixing compilation errors
    pub fn revision_prompt(
        original_prompt: &str,
        previous_spec: &ReportSpec,
        diagnostics: &CompilerDiagnostics,
        ctx: &PlannerContext,
    ) -> String {
        let schema_info = Self::format_schema_summary(&ctx.schema_summary);
        let constraints_info = Self::format_constraints(&ctx.constraints);
        let current_date = ctx.current_date.as_deref().unwrap_or("YYYY-MM-DD");
        let diagnostics_str = format!("{:?}", diagnostics);

        format!(
            r#"Previous attempt failed compilation. Fix the ReportSpec.

🗓️  CURRENT DATE: {}
When the user says "today" or "now", use this date: {}

ORIGINAL PROMPT: {}
PREVIOUS SPEC: {}
COMPILER ERRORS: {}

CONSTRAINTS:
{}

WORKSPACE: {}
{}

REQUIRED OUTPUT FORMAT:
{{
  "report_spec": {{
    "version": 1,
    "workspace": "{}",
    "select": [{{"field": "actual_field_from_schema", "alias": null}}],
    "filters": [{{"field": "field_name", "op": "eq", "value": "example_value"}}],
    "order_by": [{{"field": "field_name", "dir": "asc"}}],
    "mode": "preview",
    "pagination": null
  }},
  "assumptions": ["list any assumptions made"],
  "open_questions": ["list any unclear requirements"],
  "notes": "explanation of fixes made"
}}

CRITICAL RULES (YOU MUST FOLLOW THESE):
⚠️  SCHEMA CORRECTNESS:
1. "select" array MUST NOT be empty - include at least one field
2. ALL field names MUST exist in the SCHEMA SUMMARY above (verify each field!)
3. NEVER use field names not listed in the schema
4. If a field is used in "order_by", it MUST also appear in "select"
5. CRITICAL: Use ONLY the logical field names from the schema (e.g., "brand", "id", "name")
6. NEVER use SQL-qualified names (❌ WRONG: "o.id", "c.brand" ✅ CORRECT: "id", "brand")

⚠️  FORMAT CORRECTNESS:
5. Use lowercase for "op" values: "eq", "in", "overlaps", "gt", "gte", "lt", "lte", "is_null", "is_not_null"
6. Use lowercase for "dir" values: "asc", "desc"
7. "filters" and "order_by" can be empty arrays [] if not needed
8. Only output valid JSON - no explanations outside the JSON structure
9. CRITICAL: Close all arrays with ] before closing objects with }}
10. For long country arrays, maintain valid JSON structure

⚠️  COMMON MISTAKES TO AVOID:
- ❌ Empty "select" array
- ❌ Using fields not in schema
- ❌ ORDER BY fields missing from SELECT
- ❌ Uppercase operator names (use "eq" not "Eq")

FILTER OPERATORS (lowercase only):
- "eq": Equal to (for NULL checks use "eq" with value: null, generates IS NULL)
- "in": In list (value must be array)
- "overlaps": Overlaps with (for array fields, value must be array)
- "gt": Greater than (for dates, numbers)
- "gte": Greater than or equal (for dates, numbers)
- "lt": Less than (for dates, numbers)
- "lte": Less than or equal (for dates, numbers)
- "is_null": Check if field is NULL (no value needed)
- "is_not_null": Check if field is NOT NULL (no value needed)

NULL CHECKS:
- To check if field IS NULL: {{"field": "fieldName", "op": "is_null", "value": null}}
- To check if field IS NOT NULL: {{"field": "fieldName", "op": "is_not_null", "value": null}}
- Alternative: use "eq" with null value for NULL checks

DATE HANDLING (CRITICAL - READ THE CURRENT DATE AT THE TOP OF THIS PROMPT):
⚠️  "TODAY" or "NOW" QUERIES - USE THE CURRENT DATE SHOWN AT THE TOP:
When user asks for campaigns/offers that are "live today" or "active now":
- READ THE CURRENT DATE AT THE TOP OF THIS PROMPT
- YOU MUST USE THAT EXACT DATE in your filters
- WRONG EXAMPLES: "2025-01-01", "2021-12-10", "2024-12-31" (these are all WRONG)
- CORRECT: Use the date shown at the very top of this prompt where it says "🗓️  CURRENT DATE"
- An offer is live TODAY if:
  * startDate <= CURRENT_DATE (from the top of this prompt)
  * AND (endDate >= CURRENT_DATE OR endDate is NULL)
- NULL endDate means UNLIMITED/ONGOING - the campaign/offer never expires
- Example filters for "live today" - USE THE DATE FROM THE TOP:
  {{"field": "startDate", "op": "lte", "value": "USE-CURRENT-DATE-FROM-TOP"}},
  {{"field": "endDate", "op": "gte", "value": "USE-CURRENT-DATE-FROM-TOP"}}

⚠️  DATE INFERENCE:
When a date is mentioned WITHOUT a year (e.g., "December 10", "Jan 5"):
- Always infer the most recent occurrence relative to the CURRENT DATE shown at the top
- NEVER use arbitrary years like 2021 or 2020 or 2025
- NEVER assume future dates unless explicitly stated

⚠️  DATE RANGE QUERIES (for "live between", "active during", "running from X to Y"):
For campaigns/offers that were "live" or "active" during a period [START, END]:
- Use OVERLAP logic, NOT simple comparison
- An offer is live during [START, END] if:
  * startDate <= END (started before or during the period)
  * AND (endDate >= START OR endDate is NULL)
- NULL endDate means UNLIMITED/ONGOING
- When user says "today", use the CURRENT DATE shown at the top of this prompt
- Common mistake: Using "startDate >= START" misses campaigns that started before the period
- Common mistake: Not handling NULL endDate means missing unlimited campaigns

⚠️  CAMPAIGN/OFFER STATUS:
- "LIVE" status alone is NOT sufficient for date range queries
- A campaign can have status="LIVE" but be outside the date range
- Always combine status checks with proper date range filters

⚠️  NULL END DATE:
- NULL or missing endDate means the campaign/offer is UNLIMITED/ONGOING
- When checking "live today", NULL endDate should be treated as valid (ongoing)
- The SQL generator handles NULL endDate automatically in >= comparisons

REGION TO COUNTRY MAPPING:
When users mention regions, expand them to ISO 3166-1 alpha-2 country codes:
- APAC (Asia-Pacific): ["AF","AU","BD","BT","BN","KH","CN","HK","IN","ID","JP","KI","KP","KR","LA","MY","MV","MN","MM","NP","NZ","PK","PG","PH","SG","SB","LK","TW","TH","TL","VU","VN"]
- EMEA (Europe/Middle East/Africa): ["AL","DZ","AD","AO","AM","AT","AZ","BH","BY","BE","BA","BW","BG","BI","CM","CV","CF","TD","KM","CG","HR","CY","CZ","DK","DJ","EG","GQ","ER","EE","ET","FI","FR","GA","GM","GE","DE","GH","GR","GN","GW","HU","IS","IR","IQ","IE","IL","IT","CI","JO","KZ","KE","KW","KG","LV","LB","LS","LR","LY","LI","LT","LU","MK","MG","MW","ML","MT","MR","MU","MD","MC","ME","MA","MZ","NA","NL","NE","NG","NO","OM","PS","PL","PT","QA","RO","RU","RW","ST","SA","SN","RS","SC","SL","SK","SI","SO","ZA","SS","ES","SD","SZ","SE","CH","SY","TJ","TZ","TG","TN","TR","TM","UG","UA","AE","GB","UZ","VA","YE","ZM","ZW"]
- LATAM (Latin America): ["AR","BZ","BO","BR","CL","CO","CR","CU","DO","EC","SV","GT","HT","HN","MX","NI","PA","PY","PE","UY","VE"]
- NA (North America): ["US","CA"]

IMPORTANT: Always use ISO 3166-1 alpha-2 country codes:
- Use "GB" for United Kingdom (NOT "UK")
- Use "US" for United States (NOT "USA")
- Never use region names as literal values in filters

DISAMBIGUATING NAME FIELDS (CRITICAL):
⚠️  Multiple entities have a "name" field - ALWAYS use explicit names:
- For offer name: use "offer_name" (NOT just "name")
- For product name: use "product_name" (NOT just "name")
- For campaign name: use "campaign_name"
- NEVER use bare "name" field when products or campaigns are involved

Examples:
- "Show offer name and product name" →
  {{"field": "offer_name"}}, {{"field": "product_name"}}
- "List offer id, offer name, product id, product name" →
  {{"field": "offer_id"}}, {{"field": "offer_name"}}, {{"field": "product_id"}}, {{"field": "product_name"}}

CAMPAIGN vs OFFER FIELDS:
When users ask for both campaign and offer data:
- Campaign fields use "campaign_" prefix: "campaign_id", "campaign_name", "campaign_startDate", "campaign_endDate"
- Offer fields can use explicit prefix: "offer_id", "offer_name" OR no prefix: "id" (when no ambiguity)
- Example: "Campaign ID and name, offer id and name" →
  {{"field": "campaign_id"}}, {{"field": "campaign_name"}}, {{"field": "offer_id"}}, {{"field": "offer_name"}}

BRAND FILTERING (ESPN, DISNEY, STAR, HULU):
When users ask for offers by brand (ESPN, DISNEY, STAR, HULU):
- Use the "brand" field in filters: {{"field": "brand", "op": "eq", "value": "ESPN"}}
- Brand filtering is case-insensitive - you can use any case (e.g., "espn", "ESPN", "Espn")
- The database uses case-insensitive comparison (ILIKE/LOWER), so case doesn't matter
- Common brand values: "ESPN", "DISNEY", "STAR", "HULU"
- The system will automatically handle joins between campaigns and offers
- Example: "ESPN offers" → {{"field": "brand", "op": "eq", "value": "espn"}}
- Example: "disney or hulu" → {{"field": "brand", "op": "in", "value": ["disney", "hulu"]}}

PRICE TYPE FILTERING (RETAIL, etc.):
⚠️  CRITICAL: When users mention "retail offers" or "retail pricing":
- "retail" refers to the "priceType" field, NOT a generic description
- Use the "priceType" field with value "RETAIL": {{"field": "priceType", "op": "eq", "value": "RETAIL"}}
- Common priceType values: "RETAIL" (most common)
- Example: "retail offers" → {{"field": "priceType", "op": "eq", "value": "RETAIL"}}
- Example: "Find retail offers in South Korea" → filters include {{"field": "priceType", "op": "eq", "value": "RETAIL"}}
- DO NOT confuse with other uses of "retail" in the user query
- The priceType field is in offers_latest.attributes

CHECKING IF PRICE IS DEFINED:
⚠️  CRITICAL: When users ask for offers "where price is defined" or "with prices":
- Check if the offer has a price_id: {{"field": "price_id", "op": "is_not_null", "value": null}}
- DO NOT check if "amount" is not null - that checks the prices table, not the offer
- "price is defined on offer" = offer_products.price_id IS NOT NULL
- "price is not defined" = offer_products.price_id IS NULL
- Examples:
  - "offers where price is defined" → {{"field": "price_id", "op": "is_not_null", "value": null}}
  - "offers with prices" → {{"field": "price_id", "op": "is_not_null", "value": null}}
  - "offers without prices" → {{"field": "price_id", "op": "is_null", "value": null}}

IMPORTANT: Fix the errors and output only valid JSON. No explanations outside the JSON structure."#,
            current_date,
            current_date,
            original_prompt,
            serde_json::to_string_pretty(previous_spec).unwrap_or_default(),
            diagnostics_str,
            constraints_info,
            ctx.workspace,
            schema_info,
            ctx.workspace
        )
    }

    /// Generate user prompt with natural language request
    pub fn user_prompt(natural_language: &str) -> String {
        format!(
            "Generate a ReportSpec for this request: {}",
            natural_language
        )
    }

    /// Format schema summary for LLM context
    fn format_schema_summary(schema: &SchemaSummary) -> String {
        let mut result = String::from("SCHEMA SUMMARY:\n");
        result.push_str("⚠️  IMPORTANT: Use field names directly (e.g., 'id', 'name'). DO NOT prefix with table name or alias!\n\n");

        // Format tables and fields
        for table in &schema.tables {
            result.push_str(&format!(
                "Table: {} (SQL alias for internal use: {})\n",
                table.name, table.alias
            ));
            if let Some(desc) = &table.description {
                result.push_str(&format!("  Description: {}\n", desc));
            }
            result.push_str("  Available Fields (use these names directly, NO table prefix):\n");
            for field in &table.fields {
                let nullable = if field.nullable {
                    "nullable"
                } else {
                    "required"
                };
                result.push_str(&format!(
                    "    - {} ({}, {})\n",
                    field.name, field.field_type, nullable
                ));
                if let Some(desc) = &field.description {
                    result.push_str(&format!("      Description: {}\n", desc));
                }
                if let Some(enum_vals) = &field.enum_values {
                    result.push_str(&format!("      Values: [{}]\n", enum_vals.join(", ")));
                }
            }
            result.push('\n');
        }

        // Format enums
        if !schema.enums.is_empty() {
            result.push_str("ENUMS:\n");
            for enum_def in &schema.enums {
                result.push_str(&format!(
                    "  {}: [{}]\n",
                    enum_def.name,
                    enum_def.values.join(", ")
                ));
                if let Some(desc) = &enum_def.description {
                    result.push_str(&format!("    Description: {}\n", desc));
                }
            }
        }

        result
    }

    /// Format examples for LLM context
    fn format_examples(examples: &[super::schema_summary::ExamplePair]) -> String {
        if examples.is_empty() {
            return String::new();
        }

        let mut result = String::from("EXAMPLES:\n");
        for (i, example) in examples.iter().enumerate() {
            result.push_str(&format!("Example {}:\n", i + 1));
            result.push_str(&format!("  Prompt: {}\n", example.prompt));
            result.push_str(&format!("  Description: {}\n", example.description));
            if let Ok(spec_json) = serde_json::to_string_pretty(&example.spec) {
                result.push_str(&format!("  ReportSpec: {}\n", spec_json));
            }
            result.push('\n');
        }

        result
    }

    /// Format constraints for LLM context
    fn format_constraints(constraints: &super::schema_summary::PlannerConstraints) -> String {
        let mut result = String::new();

        if constraints.max_select_fields > 0 {
            result.push_str(&format!(
                "- Maximum {} select fields\n",
                constraints.max_select_fields
            ));
        }
        if constraints.max_filters > 0 {
            result.push_str(&format!("- Maximum {} filters\n", constraints.max_filters));
        }
        if !constraints.forbidden_patterns.is_empty() {
            result.push_str(&format!(
                "- Forbidden patterns: {}\n",
                constraints.forbidden_patterns.join(", ")
            ));
        }

        result
    }
}
