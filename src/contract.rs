use std::ops::Add;

#[cfg(not(feature = "library"))]
use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdError,
    StdResult,
};
use cw2::set_contract_version;
use cw_storage_plus::Bound;
// use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{EntryResponse, ExecuteMsg, InstantiateMsg, ListResponse, QueryMsg};
use crate::state::{Config, Entry, Priority, Status, CONFIG, ENTRY_SEQ, LIST};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:todo-list-smartcontract";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(_deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let owner = _msg
        .owner
        .and_then(|addr_string| _deps.api.addr_validate(addr_string.as_str()).ok())
        .unwrap_or(_info.sender);

    let config = Config {
        owner: owner.clone(),
    };
    CONFIG.save(_deps.storage, &config)?;
    ENTRY_SEQ.save(_deps.storage, &0u64)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", owner))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match _msg {
        ExecuteMsg::NewEntry {
            description,
            priority,
        } => execute_create_new_entry(_deps, _info, description, priority),
        ExecuteMsg::UpdateEntry {
            id,
            description,
            status,
            priority,
        } => execute_update_entry(_deps, _info, id, description, status, priority),
        ExecuteMsg::DeleteEntry { id } => execute_delete_entry(_deps, _info, id),
        ExecuteMsg::TransferOwnerShip { new_owner } => transfer_ownership(_deps, _info, new_owner),
    }
}

pub fn execute_create_new_entry(
    deps: DepsMut,
    info: MessageInfo,
    description: String,
    priority: Option<Priority>,
) -> Result<Response, ContractError> {
    let owner = CONFIG.load(deps.storage)?.owner;
    if info.sender != owner {
        return Err(ContractError::Unauthorized {});
    }
    let id = ENTRY_SEQ.update::<_, cosmwasm_std::StdError>(deps.storage, |id| Ok(id.add(1)))?;

    let new_entry = Entry {
        id,
        description,
        priority: priority.unwrap_or(Priority::None),
        status: Status::Todo,
    };

    LIST.save(deps.storage, id, &new_entry)?;

    Ok(Response::new()
        .add_attribute("method", "execute_create_new_entry")
        .add_attribute("new_entry_value", id.to_string()))
}

pub fn execute_update_entry(
    deps: DepsMut,
    info: MessageInfo,
    id: u64,
    description: Option<String>,
    status: Option<Status>,
    priority: Option<Priority>,
) -> Result<Response, ContractError> {
    let owner = CONFIG.load(deps.storage)?.owner;

    if info.sender != owner {
        return Err(ContractError::Unauthorized {});
    }

    let entry = LIST.load(deps.storage, id)?;

    let updated_entry = Entry {
        id,
        description: description.unwrap_or(entry.description),
        status: status.unwrap_or(entry.status),
        priority: priority.unwrap_or(entry.priority),
    };

    LIST.save(deps.storage, id, &updated_entry)?;

    Ok(Response::new()
        .add_attribute("method", "execute_update_entry")
        .add_attribute("updated_entry_id", id.to_string()))
}

pub fn execute_delete_entry(
    deps: DepsMut,
    info: MessageInfo,
    id: u64,
) -> Result<Response, ContractError> {
    let owner = CONFIG.load(deps.storage)?.owner;
    if info.sender != owner {
        return Err(ContractError::Unauthorized {});
    }
    LIST.remove(deps.storage, id);

    Ok(Response::new()
        .add_attribute("method", "execute_delete_entry")
        .add_attribute("deleted_entry_id", id.to_string()))
}

pub fn transfer_ownership(
    deps: DepsMut,
    info: MessageInfo,
    new_owner: String,
) -> Result<Response, ContractError> {
    let owner = CONFIG.load(deps.storage)?.owner;
    if info.sender != owner {
        return Err(ContractError::Unauthorized {});
    }

    let new_owner =
        Some(new_owner).and_then(|addr_string| deps.api.addr_validate(addr_string.as_str()).ok());

    let config = match new_owner {
        Some(x) => Config { owner: x },
        None => {
            return Err(ContractError::Std(StdError::ParseErr {
                target_type: "Addr".to_string(),
                msg: "invalid parse owner".to_string(),
            }))
        }
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("method", "transfer_owner")
        .add_attribute("new_owner", config.owner.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    match _msg {
        QueryMsg::QueryEntry { id } => to_binary(&query_entry(_deps, id)?),
        QueryMsg::QueryList { start_after, limit } => {
            to_binary(&query_list(_deps, start_after, limit)?)
        }
    }
}

pub fn query_entry(deps: Deps, id: u64) -> StdResult<EntryResponse> {
    // The entry with the matching `id` is loaded from the `LIST`.
    let entry = LIST.load(deps.storage, id)?;
    // An `EntryResponse` is formed with the attributes of the loaded entry and returned.
    Ok(EntryResponse {
        id: entry.id,
        description: entry.description,
        status: entry.status,
        priority: entry.priority,
    })
}

const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

fn query_list(deps: Deps, start_after: Option<u64>, limit: Option<u32>) -> StdResult<ListResponse> {
    let start = start_after.map(Bound::exclusive);
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    let entries: StdResult<Vec<_>> = LIST
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .collect();

    let result = ListResponse {
        entries: entries?.into_iter().map(|l| l.1.into()).collect(),
    };

    Ok(result)
}

#[cfg(test)]
mod tests {}
