import { Body, Controller, Delete, Get, Param, Patch, Post, UseGuards } from "@nestjs/common";
import { ActorGuard, AllowedActors } from "../../common/actor.js";
import { ProvidersService, UpsertProviderDto } from "./providers.service.js";

@Controller("providers")
@UseGuards(ActorGuard)
@AllowedActors("user") // 只有真人管理员能改模型接入
export class ProvidersController {
  constructor(private readonly providers: ProvidersService) {}

  @Get()
  list() {
    return this.providers.list().then((data) => ({ data }));
  }

  @Post()
  create(@Body() body: UpsertProviderDto) {
    return this.providers.create(body).then((data) => ({ data }));
  }

  @Post(":id/test")
  test(@Param("id") id: string) {
    return this.providers.test(id).then((data) => ({ data }));
  }

  @Patch(":id/enabled")
  setEnabled(@Param("id") id: string, @Body() body: { enabled: boolean }) {
    return this.providers.setEnabled(id, body.enabled).then((data) => ({ data }));
  }

  @Delete(":id")
  remove(@Param("id") id: string) {
    return this.providers.remove(id).then((data) => ({ data }));
  }
}
