import { Body, Controller, Delete, Get, Param, Post, UseGuards } from "@nestjs/common";
import { ActorGuard, AllowedActors } from "../../common/actor.js";
import { TeamsService, UpsertTeamDto } from "./teams.service.js";

@Controller("teams")
@UseGuards(ActorGuard)
export class TeamsController {
  constructor(private readonly teams: TeamsService) {}

  @Get()
  list() {
    return this.teams.list().then((data) => ({ data }));
  }

  @Get(":id")
  get(@Param("id") id: string) {
    return this.teams.get(id).then((data) => ({ data }));
  }

  @Post()
  @AllowedActors("user")
  create(@Body() body: UpsertTeamDto) {
    return this.teams.create(body).then((data) => ({ data }));
  }

  @Delete(":id")
  @AllowedActors("user")
  remove(@Param("id") id: string) {
    return this.teams.remove(id).then((data) => ({ data }));
  }
}
